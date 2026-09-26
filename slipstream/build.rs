fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = vergen_gitcl::Build::all_build();
    let rustc = vergen_gitcl::Rustc::all_rustc();
    let cargo = vergen_gitcl::Cargo::all_cargo();
    let gitcl = vergen_gitcl::Gitcl::all_git();
    let si = vergen_gitcl::Sysinfo::all_sysinfo();

    vergen_gitcl::Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&rustc)?
        .add_instructions(&gitcl)?
        .add_instructions(&cargo)?
        .add_instructions(&si)?
        .emit()?;

    Ok(())
}
