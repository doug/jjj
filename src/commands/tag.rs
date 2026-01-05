use crate::cli::TagAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

pub fn execute(action: TagAction) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    match action {
        TagAction::List { json } => list_tags(&store, json),
        TagAction::New { name, desc, color } => create_tag(&store, name, desc, color),
        TagAction::Edit {
            tag_id,
            name,
            desc,
            color,
        } => edit_tag(&store, tag_id, name, desc, color),
        TagAction::Delete { tag_id } => delete_tag(&store, tag_id),
    }
}

fn list_tags(store: &MetadataStore, json: bool) -> Result<()> {
    let config = store.load_config()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&config.tags)?);
        return Ok(());
    }

    if config.tags.is_empty() {
        println!("No tags found.");
        return Ok(());
    }

    println!("Tags:");
    for tag in config.tags {
        let desc = tag.description.as_deref().unwrap_or("");
        let color = tag.color.as_deref().unwrap_or("");
        println!("  {} - {} ({}) {}", tag.id, tag.name, desc, color);
    }

    Ok(())
}

fn create_tag(
    store: &MetadataStore,
    name: String,
    description: Option<String>,
    color: Option<String>,
) -> Result<()> {
    store.with_metadata(&format!("Create tag {}", name), || {
        let mut config = store.load_config()?;
        let tag = config.add_tag(name.clone(), description.clone(), color.clone());
        store.save_config(&config)?;
        println!("Created tag {} ({})", tag.id, tag.name);
        Ok(())
    })
}

fn edit_tag(
    store: &MetadataStore,
    tag_id: String,
    name: Option<String>,
    description: Option<String>,
    color: Option<String>,
) -> Result<()> {
    store.with_metadata(&format!("Edit tag {}", tag_id), || {
        let mut config = store.load_config()?;
        
        if let Some(tag) = config.tags.iter_mut().find(|t| t.id == tag_id) {
            if let Some(n) = name {
                tag.name = n;
            }
            if let Some(d) = description {
                tag.description = Some(d);
            }
            if let Some(c) = color {
                tag.color = Some(c);
            }
            store.save_config(&config)?;
            println!("Updated tag {}", tag_id);
        } else {
            return Err(format!("Tag {} not found", tag_id).into());
        }
        
        Ok(())
    })
}

fn delete_tag(store: &MetadataStore, tag_id: String) -> Result<()> {
    store.with_metadata(&format!("Delete tag {}", tag_id), || {
        let mut config = store.load_config()?;
        
        if let Some(pos) = config.tags.iter().position(|t| t.id == tag_id) {
            config.tags.remove(pos);
            store.save_config(&config)?;
            println!("Deleted tag {}", tag_id);
        } else {
            return Err(format!("Tag {} not found", tag_id).into());
        }
        
        Ok(())
    })
}
