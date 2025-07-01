use crate::config::DiffCommand;

#[allow(dead_code)]
pub struct DiffExecutor {
    command: DiffCommand,
}

#[allow(dead_code)]
impl DiffExecutor {
    pub fn new(command: DiffCommand) -> Self {
        Self { command }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DiffCommand;

    #[test]
    fn test_diff_executor_new() {
        let cmd = DiffCommand::default();
        let executor = DiffExecutor::new(cmd);
        assert_eq!(executor.command.command, "diff");
    }
}
