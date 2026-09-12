use xtask_codegen::{TaskCommand, generate_ast, task_command};
use xtask_glue::{Mode::Overwrite, Result, project_root, pushd};

fn main() -> Result<()> {
    let _d = pushd(project_root());
    let result = task_command().fallback_to_usage().run();

    match result {
        TaskCommand::Grammar(language_list) => {
            generate_ast(Overwrite, language_list)?;
        },
        TaskCommand::All => {
            generate_ast(Overwrite, vec![])?;
        },
    }

    Ok(())
}
