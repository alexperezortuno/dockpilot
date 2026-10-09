pub mod client;

use std::{ffi::OsString, fs::File, io, path::PathBuf, process::Stdio};
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct CommandSpec {
    program: OsString,
    args: Vec<OsString>,
    current_dir: Option<PathBuf>,
    stdin_path: Option<PathBuf>,
    stdout_path: Option<PathBuf>,
}

impl CommandSpec {
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            current_dir: None,
            stdin_path: None,
            stdout_path: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    #[allow(dead_code)]
    pub fn current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(path.into());
        self
    }

    pub fn stdin_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.stdin_path = Some(path.into());
        self
    }

    pub fn stdout_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.stdout_path = Some(path.into());
        self
    }

    pub(crate) fn display(&self) -> String {
        let mut parts = vec![self.program.to_string_lossy().into_owned()];
        parts.extend(
            self.args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned()),
        );
        parts.join(" ")
    }
}

#[derive(Debug)]
pub struct CommandResult {
    pub lines: Vec<String>,
    pub stdout: Vec<u8>,
    pub success: bool,
}

fn failed_result(display: &str, error: io::Error) -> CommandResult {
    CommandResult {
        lines: vec![
            format!("$ {}", display),
            format!("[Error executing command: {}]", error),
            String::new(),
        ],
        stdout: Vec::new(),
        success: false,
    }
}

pub async fn run_command(spec: CommandSpec) -> CommandResult {
    let display = spec.display();
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);

    if let Some(path) = &spec.current_dir {
        command.current_dir(path);
    }
    if let Some(path) = &spec.stdin_path {
        match File::open(path) {
            Ok(file) => {
                command.stdin(Stdio::from(file));
            }
            Err(error) => return failed_result(&display, error),
        }
    }
    if let Some(path) = &spec.stdout_path {
        match File::create(path) {
            Ok(file) => {
                command.stdout(Stdio::from(file));
            }
            Err(error) => return failed_result(&display, error),
        }
    }

    let output = command.kill_on_drop(true).output().await;

    match output {
        Ok(output) => {
            let mut lines = vec![format!("$ {}", display)];
            append_output(&mut lines, &output);
            lines.push(String::new());
            CommandResult {
                lines,
                stdout: output.stdout,
                success: output.status.success(),
            }
        }
        Err(error) => failed_result(&display, error),
    }
}

fn append_output(output_lines: &mut Vec<String>, output: &std::process::Output) {
    if !output.stdout.is_empty() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            output_lines.push(line.to_string());
        }
    }
    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            output_lines.push(format!("[stderr] {}", line));
        }
    }
    if !output.status.success() {
        output_lines.push(format!("[exit code: {:?}]", output.status.code()));
    }
}

pub async fn run_stop_all() -> Vec<CommandResult> {
    let list_spec = CommandSpec::new("docker").args(["ps", "-aq"]);
    let list_result = run_command(list_spec).await;
    if !list_result.success {
        return vec![list_result];
    }

    let mut results = vec![list_result];
    let ids = String::from_utf8_lossy(&results[0].stdout).into_owned();
    for id in ids.lines() {
        if !id.is_empty() {
            results.push(run_command(CommandSpec::new("docker").args(["stop", id])).await);
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::CommandSpec;
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn arguments_preserve_shell_metacharacters() {
        let spec = CommandSpec::new("docker").arg("name; touch injected");

        assert_eq!(spec.args, vec![OsString::from("name; touch injected")]);
    }

    #[test]
    fn file_paths_remain_single_explicit_paths() {
        let path = PathBuf::from("project files/image archive.tar");
        let spec = CommandSpec::new("docker")
            .arg("load")
            .stdin_file(path.clone())
            .current_dir("project files");

        assert_eq!(spec.args, vec![OsString::from("load")]);
        assert_eq!(spec.stdin_path, Some(path));
        assert_eq!(spec.current_dir, Some(PathBuf::from("project files")));
    }
}
