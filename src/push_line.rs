use crate::CRLF;

pub(crate) trait PushLine {
    fn push_line(&mut self, line: &str);
}

impl PushLine for String {
    fn push_line(&mut self, line: &str) {
        self.push_str(line);
        self.push_str(CRLF);
    }
}
