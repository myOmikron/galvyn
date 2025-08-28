use std::fs::create_dir;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::FileExt;
use std::path::Path;

use anyhow::anyhow;
use anyhow::Context;
use convert_case::Case;
use convert_case::Casing;
use include_dir::include_dir;
use include_dir::Dir;
use include_dir::DirEntry;
use tera::Tera;

use crate::output::print_info;

static CRATE_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/crate");

/// Initialize a new galvyn project
pub fn run_init(name: String, path: String) -> anyhow::Result<()> {
    let p = Path::new(&path);

    if !p.is_dir() {
        return Err(anyhow!("{path} does not exist"));
    }

    let mut ctx = tera::Context::new();
    ctx.insert("crate_name", &name);

    create_dir(p.join(&name)).with_context(|| "Couldn't create initial directory")?;

    // Recursively create structure and evaluate tera templates
    create_entries(&ctx, &p.join(&name), CRATE_DIR.entries())
        .with_context(|| "Error evaluating and writing templates")?;

    Ok(())
}

static MOD_DIR: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/templates/module");

/// Create a new module
pub fn run_module(name: String, path: String) -> anyhow::Result<()> {
    let p = Path::new(&path);

    if !p.is_dir() {
        return Err(anyhow!("{path} does not exist"));
    }

    validate_name(&name).with_context(|| format!("Validation error of module name \"{name}\""))?;

    let mut ctx = tera::Context::new();
    ctx.insert("mod_name", &name.to_case(Case::Pascal));

    create_dir(p.join(&name)).with_context(|| "Couldn't create initial module")?;

    // Recursively create structure and evaluate tera templates
    create_entries(&ctx, &p.join(&name), MOD_DIR.entries())
        .with_context(|| "Error evaluating and writing templates")?;

    attach_mod(&name, p).with_context(|| format!("Could not attach module {name}"))?;

    Ok(())
}

/// Validate the name of the module
fn validate_name(name: &str) -> anyhow::Result<()> {
    syn::parse_str::<syn::Ident>(name)?;

    Ok(())
}

fn create_entries(ctx: &tera::Context, path: &Path, curr: &[DirEntry]) -> anyhow::Result<()> {
    for entry in curr {
        match entry {
            DirEntry::Dir(dir) => {
                create_dir(path.join(dir.path())).with_context(|| {
                    format!(
                        "Could not create directory {}",
                        path.join(dir.path()).display()
                    )
                })?;
                create_entries(ctx, path, dir.entries())?;
            }
            DirEntry::File(file) => {
                let file_path = &path.join(file.path());

                let mut new_file = File::create(file_path)
                    .with_context(|| format!("Could not create file {}", file_path.display()))?;
                let content = file
                    .contents_utf8()
                    .ok_or(anyhow!("Template was not utf8"))?;

                let mut tera = Tera::default();
                let rendered = tera.render_str(content, ctx)?;

                new_file
                    .write_all(rendered.as_bytes())
                    .with_context(|| format!("Could not write to {}", file_path.display()))?;
            }
        }
    }

    Ok(())
}

/// Attach a created module to a parent module
fn attach_mod(name: &str, path: &Path) -> anyhow::Result<()> {
    let p_mod = path.join("mod.rs");
    let p_main = path.join("main.rs");
    let p_lib = path.join("lib.rs");

    let chosen_mod;

    let mut open_options = OpenOptions::new();
    open_options.append(true);

    let file = if p_mod.exists() && p_mod.is_file() {
        chosen_mod = p_mod.display().to_string();
        open_options.open(p_mod)?
    } else if p_main.exists() && p_main.is_file() {
        chosen_mod = p_main.display().to_string();
        open_options.open(p_main)?
    } else if p_lib.exists() && p_lib.is_file() {
        chosen_mod = p_lib.display().to_string();
        open_options.open(p_lib)?
    } else {
        return Err(anyhow!("Parent module was not found"));
    };

    let mod_line = format!("pub mod {name};\n");
    file.write_all_at(mod_line.as_bytes(), 0)
        .with_context(|| "Could not write to parent module")?;

    print_info(&format!("Attached {name} to {chosen_mod}"));

    Ok(())
}
