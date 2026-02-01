use crate::error::Result;
use crate::jj::JjClient;

pub fn execute(
    bookmarks: Vec<String>,
    remote: &str,
    no_prompt: bool,
    dry_run: bool,
) -> Result<()> {
    let jj_client = JjClient::new()?;

    if dry_run {
        println!("Would push to {}:", remote);
        for b in &bookmarks {
            println!("  {}", b);
        }
        println!("  jjj/meta");
        return Ok(());
    }

    // 1. Push specified bookmarks
    for bookmark in &bookmarks {
        println!("Pushing {}...", bookmark);
        jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote])?;
    }

    // 2. Always push jjj/meta
    println!("Pushing jjj/meta...");
    jj_client.execute(&["git", "push", "-b", "jjj/meta", "--remote", remote])?;

    println!("Pushed to {}.", remote);

    // Smart prompts will be added in Task 4
    let _ = no_prompt;

    Ok(())
}
