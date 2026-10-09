use std::process::Command;

pub fn execute_command(output_lines: &mut Vec<String>, output_scroll: &mut u16, command: &str) {
    output_lines.push(format!("$ {}", command));
    *output_scroll = output_lines.len() as u16;

    let output = Command::new("sh").arg("-c").arg(command).output();

    match output {
        Ok(out) => {
            if !out.stdout.is_empty() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    output_lines.push(line.to_string());
                }
            }
            if !out.stderr.is_empty() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                for line in stderr.lines() {
                    output_lines.push(format!("[stderr] {}", line));
                }
            }
            if !out.status.success() {
                output_lines.push(format!("[exit code: {:?}]", out.status.code()));
            }
        }
        Err(e) => {
            output_lines.push(format!("[error ejecutando comando: {}]", e));
        }
    }

    output_lines.push(String::new());
    *output_scroll = output_lines.len() as u16;
}
