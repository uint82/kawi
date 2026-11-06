use super::App;

impl App {
    pub(crate) fn prewarm_syntax(&mut self) {
        self.refresh_syntax();
    }
}

#[cfg(test)]
mod tests;
