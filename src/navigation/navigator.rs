use std::path::{Path, PathBuf};

pub(crate) struct Navigator {
    history: Vec<PathBuf>,
    position: usize,
    pub(crate) show_hidden: bool,
}

impl Navigator {
    pub(crate) fn new(start: PathBuf, show_hidden: bool) -> Self {
        Self {
            history: vec![start],
            position: 0,
            show_hidden,
        }
    }

    pub(crate) fn current(&self) -> &Path {
        &self.history[self.position]
    }

    pub(crate) fn navigate_to(&mut self, path: PathBuf) {
        if self.current() == path {
            return;
        }
        self.history.truncate(self.position + 1);
        self.history.push(path);
        self.position = self.history.len() - 1;
    }

    pub(crate) fn go_back(&mut self) -> bool {
        if self.position == 0 {
            return false;
        }
        self.position -= 1;
        true
    }

    pub(crate) fn go_forward(&mut self) -> bool {
        if self.position + 1 >= self.history.len() {
            return false;
        }
        self.position += 1;
        true
    }

    pub(crate) fn go_up(&mut self) -> bool {
        let Some(parent) = self.current().parent().map(Path::to_path_buf) else {
            return false;
        };
        self.navigate_to(parent);
        true
    }

    pub(crate) fn can_go_back(&self) -> bool {
        self.position > 0
    }

    pub(crate) fn can_go_forward(&self) -> bool {
        self.position + 1 < self.history.len()
    }

    pub(crate) fn can_go_up(&self) -> bool {
        self.current().parent().is_some()
    }
}
