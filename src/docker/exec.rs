use bollard::{
    Docker,
    container::LogOutput,
    exec::{CreateExecOptions, ResizeExecOptions, StartExecOptions, StartExecResults},
};
use futures_util::StreamExt;
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::AsyncWrite,
    io::AsyncWriteExt,
    sync::{Mutex, Notify, mpsc},
    task::JoinHandle,
};

/// Maximum pending stdin accepted from the UI. A full queue is reported, never dropped.
pub const MAX_PENDING_INPUT: usize = 256 * 1024;
const DETACH_TIMEOUT: Duration = Duration::from_secs(2);

/// Lifecycle of an interactive exec session, mirrored by the TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecState {
    Connecting,
    Running,
    Detaching,
    Finished { exit_code: Option<i64> },
    Failed { message: String },
}

impl ExecState {
    pub fn label(&self) -> String {
        match self {
            Self::Connecting => "connecting".to_string(),
            Self::Running => "running".to_string(),
            Self::Detaching => "detaching".to_string(),
            Self::Finished { exit_code } => match exit_code {
                Some(code) => format!("finished (exit code {})", code),
                None => "finished".to_string(),
            },
            Self::Failed { message } => format!("failed: {}", message),
        }
    }

    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Finished { .. } | Self::Failed { .. })
    }
}

/// Shell or command executed inside the container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecCommand {
    Sh,
    Bash,
    Custom(Vec<String>),
}

impl ExecCommand {
    pub fn argv(&self) -> Vec<String> {
        match self {
            Self::Sh => vec!["/bin/sh".to_string()],
            Self::Bash => vec!["/bin/bash".to_string()],
            Self::Custom(argv) => argv.clone(),
        }
    }

    pub fn display(&self) -> String {
        self.argv().join(" ")
    }
}

/// Split a custom command line into argv without invoking a shell.
pub fn split_exec_args(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for character in input.chars() {
        match (quote, character) {
            (Some(active), c) if c == active => quote = None,
            (Some(_), c) => current.push(c),
            (None, '\'' | '"') => quote = Some(character),
            (None, c) if c.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            (None, c) => current.push(c),
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".to_string());
    }
    if !current.is_empty() {
        args.push(current);
    }
    if args.is_empty() {
        return Err("empty command".to_string());
    }
    Ok(args)
}

#[derive(Debug)]
pub enum ExecEvent {
    Output(Vec<u8>),
    State(ExecState),
    Detached,
}

#[derive(Debug)]
pub enum ExecControl {
    Input(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Detach,
}

/// A live exec session bound to the Docker client it was created with.
pub struct ExecSession {
    pub container_id: String,
    pub container_name: String,
    pub command_display: String,
    events: mpsc::Receiver<ExecEvent>,
    control: mpsc::Sender<ExecControl>,
    input: Arc<Mutex<InputQueue>>,
    input_ready: Arc<Notify>,
    detach_requested: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

#[derive(Default)]
struct InputQueue {
    chunks: VecDeque<Vec<u8>>,
    bytes: usize,
}

impl InputQueue {
    fn push(&mut self, bytes: Vec<u8>) -> bool {
        if bytes.is_empty() || self.bytes + bytes.len() > MAX_PENDING_INPUT {
            return bytes.is_empty();
        }
        self.bytes += bytes.len();
        self.chunks.push_back(bytes);
        true
    }

    fn pop(&mut self) -> Option<Vec<u8>> {
        let bytes = self.chunks.pop_front()?;
        self.bytes -= bytes.len();
        Some(bytes)
    }
}

impl ExecSession {
    pub fn start(
        client: Docker,
        container_id: String,
        container_name: String,
        command: ExecCommand,
        rows: u16,
        cols: u16,
    ) -> Self {
        let (event_sender, events) = mpsc::channel(256);
        let (control, control_rx) = mpsc::channel(8);
        let input = Arc::new(Mutex::new(InputQueue::default()));
        let input_ready = Arc::new(Notify::new());
        let detach_requested = Arc::new(AtomicBool::new(false));
        let command_display = command.display();
        let task_container = container_id.clone();
        let handle = tokio::spawn(run(
            client,
            task_container,
            command,
            rows,
            cols,
            event_sender,
            control_rx,
            input.clone(),
            input_ready.clone(),
            detach_requested.clone(),
        ));
        Self {
            container_id,
            container_name,
            command_display,
            events,
            control,
            input,
            input_ready,
            detach_requested,
            handle,
        }
    }

    pub fn try_next_event(&mut self) -> Option<ExecEvent> {
        self.events.try_recv().ok()
    }

    /// Non-blocking input enqueue. A full bounded queue is reported to the caller.
    pub fn send(&self, control: ExecControl) -> bool {
        match control {
            ExecControl::Input(bytes) => {
                let accepted = self
                    .input
                    .try_lock()
                    .is_ok_and(|mut queue| queue.push(bytes));
                if accepted {
                    self.input_ready.notify_one();
                }
                accepted
            }
            ExecControl::Detach => {
                self.detach_requested.store(true, Ordering::Release);
                self.input_ready.notify_one();
                true
            }
            control => self.control.try_send(control).is_ok(),
        }
    }
}

impl Drop for ExecSession {
    fn drop(&mut self) {
        self.detach_requested.store(true, Ordering::Release);
        self.input_ready.notify_one();
        let handle = std::mem::replace(&mut self.handle, tokio::spawn(async {}));
        tokio::spawn(async move {
            let mut handle = handle;
            if tokio::time::timeout(DETACH_TIMEOUT, &mut handle)
                .await
                .is_err()
            {
                handle.abort();
                let _ = handle.await;
            }
        });
    }
}

struct Attached {
    exec_id: String,
    output:
        Pin<Box<dyn futures_util::Stream<Item = Result<LogOutput, bollard::errors::Error>> + Send>>,
    input: Pin<Box<dyn AsyncWrite + Send>>,
}

enum SessionEnd {
    Detached,
    StreamClosed,
    Failed(String),
    ViewDropped,
}

fn is_missing_program(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("no such file or directory")
        || lower.contains("executable file not found")
        || lower.contains("file does not exist")
}

async fn attach(
    client: &Docker,
    container_id: &str,
    command: &ExecCommand,
    rows: u16,
    cols: u16,
) -> Result<Attached, String> {
    let config = CreateExecOptions {
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(true),
        env: Some(vec!["TERM=xterm-256color".to_string()]),
        cmd: Some(command.argv()),
        ..Default::default()
    };
    let created = client
        .create_exec(container_id, config)
        .await
        .map_err(|error| error.to_string())?;
    let started = client
        .start_exec(
            &created.id,
            Some(StartExecOptions {
                detach: false,
                tty: true,
                output_capacity: Some(64 * 1024),
            }),
        )
        .await
        .map_err(|error| error.to_string())?;
    let StartExecResults::Attached { output, input } = started else {
        return Err("exec session did not attach".to_string());
    };
    // The PTY starts with a default size; align it with the current view.
    let _ = client
        .resize_exec(
            &created.id,
            ResizeExecOptions {
                height: rows.max(1),
                width: cols.max(1),
            },
        )
        .await;
    Ok(Attached {
        exec_id: created.id,
        output,
        input,
    })
}

async fn drive(
    mut attached: Attached,
    client: &Docker,
    events: &mpsc::Sender<ExecEvent>,
    control: &mut mpsc::Receiver<ExecControl>,
    input: &Arc<Mutex<InputQueue>>,
    input_ready: &Arc<Notify>,
    detach_requested: &Arc<AtomicBool>,
) -> SessionEnd {
    loop {
        tokio::select! {
            biased;
            message = control.recv() => {
                match message {
                    Some(ExecControl::Detach) => return SessionEnd::Detached,
                    Some(ExecControl::Resize { rows, cols }) => {
                        if let Err(error) = client.resize_exec(&attached.exec_id, ResizeExecOptions {
                            height: rows.max(1), width: cols.max(1),
                        }).await {
                            return SessionEnd::Failed(format!("resize failed: {}", error));
                        }
                    }
                    Some(ExecControl::Input(_)) => unreachable!("input uses the bounded queue"),
                    None => return SessionEnd::ViewDropped,
                }
            }
            chunk = attached.output.next() => {
                match chunk {
                    Some(Ok(output)) => {
                        if events
                            .send(ExecEvent::Output(output.into_bytes().to_vec()))
                            .await
                            .is_err()
                        {
                            return SessionEnd::ViewDropped;
                        }
                    }
                    Some(Err(error)) => return SessionEnd::Failed(error.to_string()),
                    None => return SessionEnd::StreamClosed,
                }
            }
            _ = input_ready.notified() => {
                if detach_requested.load(Ordering::Acquire) {
                    return SessionEnd::Detached;
                }
                let bytes = input.lock().await.pop();
                if let Some(bytes) = bytes {
                    if let Err(error) = attached.input.write_all(&bytes).await {
                        return SessionEnd::Failed(format!("stdin write failed: {}", error));
                    }
                    if let Err(error) = attached.input.flush().await {
                        return SessionEnd::Failed(format!("stdin flush failed: {}", error));
                    }
                    if !input.lock().await.chunks.is_empty() {
                        input_ready.notify_one();
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run(
    client: Docker,
    container_id: String,
    command: ExecCommand,
    rows: u16,
    cols: u16,
    events: mpsc::Sender<ExecEvent>,
    mut control: mpsc::Receiver<ExecControl>,
    input: Arc<Mutex<InputQueue>>,
    input_ready: Arc<Notify>,
    detach_requested: Arc<AtomicBool>,
) {
    macro_rules! send_state {
        ($state:expr) => {
            if events.send(ExecEvent::State($state)).await.is_err() {
                return;
            }
        };
    }

    send_state!(ExecState::Connecting);
    let mut attempt = command;
    loop {
        let attach_result = {
            let attach_command = attempt.clone();
            let attaching = attach(&client, &container_id, &attach_command, rows, cols);
            tokio::pin!(attaching);
            loop {
                tokio::select! {
                    biased;
                    message = control.recv() => match message {
                        Some(ExecControl::Detach) => {
                            send_state!(ExecState::Detaching);
                            let _ = events.send(ExecEvent::Detached).await;
                            return;
                        }
                        Some(ExecControl::Resize { .. }) => continue,
                        Some(ExecControl::Input(_)) => unreachable!("input uses the bounded queue"),
                        None => return,
                    },
                    _ = input_ready.notified() => {
                        if detach_requested.load(Ordering::Acquire) {
                            send_state!(ExecState::Detaching);
                            let _ = events.send(ExecEvent::Detached).await;
                            return;
                        }
                    }
                    result = &mut attaching => break result,
                }
            }
        };
        match attach_result {
            Ok(attached) => {
                send_state!(ExecState::Running);
                let exec_id = attached.exec_id.clone();
                match drive(
                    attached,
                    &client,
                    &events,
                    &mut control,
                    &input,
                    &input_ready,
                    &detach_requested,
                )
                .await
                {
                    SessionEnd::Detached => {
                        send_state!(ExecState::Detaching);
                        // Dropping the attach stream detaches without signaling the remote PTY.
                        let _ = events.send(ExecEvent::Detached).await;
                    }
                    SessionEnd::StreamClosed => {
                        let exit_code = client
                            .inspect_exec(&exec_id)
                            .await
                            .ok()
                            .and_then(|info| info.exit_code);
                        send_state!(ExecState::Finished { exit_code });
                    }
                    SessionEnd::Failed(message) => {
                        send_state!(ExecState::Failed { message });
                    }
                    SessionEnd::ViewDropped => {}
                }
                return;
            }
            Err(message) => {
                if attempt == ExecCommand::Bash && is_missing_program(&message) {
                    // bash is not guaranteed to exist; fall back to POSIX sh once.
                    attempt = ExecCommand::Sh;
                    continue;
                }
                send_state!(ExecState::Failed { message });
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        ExecCommand, ExecState, InputQueue, MAX_PENDING_INPUT, is_missing_program, split_exec_args,
    };

    #[test]
    fn custom_commands_split_without_a_shell() {
        assert_eq!(
            split_exec_args("ls -la /tmp").unwrap(),
            vec!["ls", "-la", "/tmp"]
        );
        assert_eq!(
            split_exec_args("echo 'hello world' \"a b\"").unwrap(),
            vec!["echo", "hello world", "a b"]
        );
        assert!(split_exec_args("echo 'oops").is_err());
        assert!(split_exec_args("   ").is_err());
    }

    #[test]
    fn metacharacters_stay_inside_single_arguments() {
        let args = split_exec_args("touch a; rm -rf /").unwrap();
        assert_eq!(args, vec!["touch", "a;", "rm", "-rf", "/"]);
        let piped = split_exec_args("echo hi | wc").unwrap();
        assert_eq!(piped, vec!["echo", "hi", "|", "wc"]);
    }

    #[test]
    fn missing_program_errors_are_classified_for_fallback() {
        assert!(is_missing_program(
            "OCI runtime exec failed: exec: \"/bin/bash\": no such file or directory"
        ));
        assert!(is_missing_program("executable file not found in $PATH"));
        assert!(!is_missing_program("permission denied"));
        assert!(!is_missing_program("container is not running"));
    }

    #[test]
    fn shell_commands_resolve_to_absolute_paths() {
        assert_eq!(ExecCommand::Sh.argv(), vec!["/bin/sh"]);
        assert_eq!(ExecCommand::Bash.argv(), vec!["/bin/bash"]);
        assert_eq!(ExecCommand::Bash.display(), "/bin/bash");
    }

    #[test]
    fn closed_states_are_terminal() {
        assert!(ExecState::Finished { exit_code: Some(0) }.is_closed());
        assert!(
            ExecState::Failed {
                message: "boom".to_string()
            }
            .is_closed()
        );
        assert!(!ExecState::Running.is_closed());
        assert_eq!(
            ExecState::Finished { exit_code: Some(1) }.label(),
            "finished (exit code 1)"
        );
    }

    #[test]
    fn input_queue_preserves_order_and_has_a_byte_bound() {
        let mut queue = InputQueue::default();
        assert!(queue.push(b"first".to_vec()));
        assert!(queue.push(b"second".to_vec()));
        assert_eq!(queue.pop().unwrap(), b"first");
        assert_eq!(queue.pop().unwrap(), b"second");
        assert!(!queue.push(vec![b'x'; MAX_PENDING_INPUT + 1]));
    }

    #[test]
    fn saturated_input_is_rejected_without_partial_enqueue() {
        let mut queue = InputQueue::default();
        assert!(queue.push(vec![b'a'; MAX_PENDING_INPUT]));
        assert!(!queue.push(b"overflow".to_vec()));
        assert_eq!(queue.pop().unwrap().len(), MAX_PENDING_INPUT);
        assert!(queue.pop().is_none());
    }

    #[tokio::test]
    async fn detach_during_connection_is_confirmed_without_waiting_for_attach() {
        let client = bollard::Docker::connect_with_http(
            "http://127.0.0.1:9",
            1,
            bollard::API_DEFAULT_VERSION,
        )
        .expect("client with unreachable endpoint");
        let mut session = super::ExecSession::start(
            client,
            "container-id".to_string(),
            "name".to_string(),
            ExecCommand::Sh,
            24,
            80,
        );
        assert!(session.send(super::ExecControl::Detach));
        let mut saw_detaching = false;
        let mut saw_detached = false;
        for _ in 0..100 {
            while let Some(event) = session.try_next_event() {
                match event {
                    super::ExecEvent::State(ExecState::Detaching) => saw_detaching = true,
                    super::ExecEvent::Detached => saw_detached = true,
                    _ => {}
                }
            }
            if saw_detaching && saw_detached {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(saw_detaching && saw_detached);
    }
}
