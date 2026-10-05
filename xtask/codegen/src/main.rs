use xtask_codegen::{TaskCommand, generate_ast, task_command};
use xtask_glue::{Mode, Result, project_root, pushd};

fn main() -> Result<()> {
    let _d = pushd(project_root());
    let result = task_command().fallback_to_usage().run();

    match result {
        TaskCommand::Grammar(language_list) => {
            generate_ast(Mode::Overwrite, language_list)?;
        },
        TaskCommand::All => {
            generate_ast(Mode::Overwrite, vec![])?;
        },
        TaskCommand::Check => {
            generate_ast(Mode::Verify, vec![])?;
        },
    }

    Ok(())
}
