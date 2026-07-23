use std::io::{self, Write};

pub(crate) const IO_ERROR_EXIT_CODE: i32 = 74;

pub(crate) fn write_stdout_line(text: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write_line(&mut stdout, text)
}

pub(crate) fn write_stderr_line(text: &str) {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    let _ = write_line(&mut stderr, text);
}

pub(crate) fn write_stdout_error(error: &io::Error, pretty: bool) {
    let report = serde_json::json!({
        "schema": "scena.cli_io_error.v1",
        "ok": false,
        "code": "io_error",
        "exit_class": "io",
        "exit_code": IO_ERROR_EXIT_CODE,
        "stream": "stdout",
        "error_kind": format!("{:?}", error.kind()),
        "message": error.to_string(),
        "help": "check the output stream, permissions, and free space",
    });
    let text = if pretty {
        serde_json::to_string_pretty(&report)
    } else {
        serde_json::to_string(&report)
    };
    if let Ok(text) = text {
        write_stderr_line(&text);
    }
}

pub(crate) fn write_line(writer: &mut impl Write, text: &str) -> io::Result<()> {
    let mut writer = io::BufWriter::new(writer);
    writer.write_all(text.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use super::write_line;

    struct ErrorWriter(io::ErrorKind);

    impl Write for ErrorWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(self.0))
        }
    }

    #[test]
    fn buffered_line_writer_preserves_text_and_adds_one_newline() {
        let mut output = Vec::new();
        write_line(&mut output, "{\"value\":\"line\\n\\t€\"}").expect("line writes");
        assert_eq!(output, b"{\"value\":\"line\\n\\t\xE2\x82\xAC\"}\n");
    }

    #[test]
    fn buffered_line_writer_preserves_broken_pipe_and_other_errors() {
        for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::StorageFull] {
            let error = write_line(&mut ErrorWriter(kind), "output")
                .expect_err("injected writer error must be returned");
            assert_eq!(error.kind(), kind);
        }
    }
}
