use bollard::{
    Docker,
    container::LogOutput,
    exec::{CreateExecOptions, ResizeExecOptions, StartExecOptions, StartExecResults},
};
use futures_util::StreamExt;
use std::pin::Pin;
use tokio::{io::AsyncWrite, io::AsyncWriteExt, sync::mpsc, task::JoinHandle};

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
    handle: JoinHandle<()>,
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
        let (control, control_rx) = mpsc::channel(256);
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
        ));
        Self {
            container_id,
            container_name,
            command_display,
            events,
            control,
            handle,
        }
    }

    pub fn try_next_event(&mut self) -> Option<ExecEvent> {
        self.events.try_recv().ok()
    }

    /// Non-blocking send; a full channel drops input rather than stalling the UI.
    pub fn send(&self, control: ExecControl) -> bool {
        self.control.try_send(control).is_ok()
    }
}

impl Drop for ExecSession {
    fn drop(&mut self) {
        self.handle.abort();
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
) -> SessionEnd {
    loop {
        tokio::select! {
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
            message = control.recv() => {
                match message {
                    Some(ExecControl::Input(bytes)) => {
                        if let Err(error) = attached.input.write_all(&bytes).await {
                            return SessionEnd::Failed(format!("stdin write failed: {}", error));
                        }
                        let _ = attached.input.flush().await;
                    }
                    Some(ExecControl::Resize { rows, cols }) => {
                        let _ = client
                            .resize_exec(
                                &attached.exec_id,
                                ResizeExecOptions {
                                    height: rows.max(1),
                                    width: cols.max(1),
                                },
                            )
                            .await;
                    }
                    Some(ExecControl::Detach) => return SessionEnd::Detached,
                    None => return SessionEnd::ViewDropped,
                }
            }
        }
    }
}

async fn run(
    client: Docker,
    container_id: String,
    command: ExecCommand,
    rows: u16,
    cols: u16,
    events: mpsc::Sender<ExecEvent>,
    mut control: mpsc::Receiver<ExecControl>,
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
        match attach(&client, &container_id, &attempt, rows, cols).await {
            Ok(attached) => {
                send_state!(ExecState::Running);
                let exec_id = attached.exec_id.clone();
                match drive(attached, &client, &events, &mut control).await {
                    SessionEnd::Detached => {
                        send_state!(ExecState::Detaching);
                        // Dropping the attached streams closes the connection;
                        // the remote process keeps running.
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
    use super::{ExecCommand, ExecState, is_missing_program, split_exec_args};

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
}
