use std::{
    ffi::OsString,
    fs::File,
    io,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

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

    fn display(&self) -> String {
        let mut parts = vec![self.program.to_string_lossy().into_owned()];
        parts.extend(
            self.args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned()),
        );
        parts.join(" ")
    }
}

fn run(spec: &CommandSpec) -> io::Result<Output> {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);

    if let Some(path) = &spec.current_dir {
        command.current_dir(path);
    }
    if let Some(path) = &spec.stdin_path {
        command.stdin(Stdio::from(File::open(path)?));
    }
    if let Some(path) = &spec.stdout_path {
        command.stdout(Stdio::from(File::create(path)?));
    }

    command.output()
}

fn append_output(output_lines: &mut Vec<String>, output: &Output) {
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

fn append_error(output_lines: &mut Vec<String>, error: &io::Error) {
    output_lines.push(format!("[error ejecutando comando: {}]", error));
}

fn finish_command(output_lines: &mut Vec<String>, output_scroll: &mut u16) {
    output_lines.push(String::new());
    *output_scroll = output_lines.len() as u16;
}

pub fn execute_command(output_lines: &mut Vec<String>, output_scroll: &mut u16, spec: CommandSpec) {
    output_lines.push(format!("$ {}", spec.display()));
    *output_scroll = output_lines.len() as u16;

    match run(&spec) {
        Ok(output) => append_output(output_lines, &output),
        Err(error) => append_error(output_lines, &error),
    }

    finish_command(output_lines, output_scroll);
}

pub fn stop_all(output_lines: &mut Vec<String>, output_scroll: &mut u16) {
    let list_spec = CommandSpec::new("docker").args(["ps", "-aq"]);
    output_lines.push(format!("$ {}", list_spec.display()));
    *output_scroll = output_lines.len() as u16;

    let output = match run(&list_spec) {
        Ok(output) => output,
        Err(error) => {
            append_error(output_lines, &error);
            finish_command(output_lines, output_scroll);
            return;
        }
    };

    append_output(output_lines, &output);
    if !output.status.success() {
        finish_command(output_lines, output_scroll);
        return;
    }

    finish_command(output_lines, output_scroll);
    for id in String::from_utf8_lossy(&output.stdout).lines() {
        if !id.is_empty() {
            execute_command(
                output_lines,
                output_scroll,
                CommandSpec::new("docker").args(["stop", id]),
            );
        }
    }
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
