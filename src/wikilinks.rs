#[derive(Debug, Clone, PartialEq)]
pub(super) struct DocumentLink {
    pub title: String,
    pub id: Option<String>,
    pub start: usize,
    pub end: usize,
}

pub(super) fn links(text: &str) -> Vec<DocumentLink> {
    let mut found = Vec::new();
    let mut offset = 0;
    while let Some(relative) = text[offset..].find("[[") {
        let start = offset + relative;
        let Some(close) = text[start + 2..].find("]]") else {
            break;
        };
        let end = start + 2 + close + 2;
        let content = text[start + 2..end - 2].trim();
        let (title, id) = content
            .rsplit_once('|')
            .map(|(title, id)| (title.trim(), Some(id.trim().to_owned())))
            .unwrap_or((content, None));
        if !title.is_empty() {
            found.push(DocumentLink {
                title: title.to_owned(),
                id,
                start: text[..start].chars().count(),
                end: text[..end].chars().count(),
            });
        }
        offset = end;
    }
    found
}

pub(super) fn resolve<'a>(
    link: &DocumentLink,
    block: &super::Block,
    workspace: &'a super::Workspace,
) -> Option<&'a super::workspace::WorkspaceDocument> {
    let id = link.id.as_deref().or_else(|| {
        block
            .link_ids
            .iter()
            .find(|(title, _)| title == &link.title)
            .map(|(_, id)| id.as_str())
    });
    if let Some(id) = id {
        workspace.documents.iter().find(|d| d.id == id)
    } else {
        workspace.by_title(&link.title)
    }
}
// TextEdit always receives readable [[Title]] text. Saved documents carry the
// immutable ID inside the brackets, with formatting offsets adjusted as well.
pub(super) fn store(block: &mut super::Block, workspace: &super::Workspace) {
    let found = links(&block.text);
    for link in found.into_iter().rev() {
        if let Some(target) = resolve(&link, block, workspace) {
            let title = target.document.title.clone();
            let id = target.id.clone();
            replace(block, &link, &format!("[[{title}|{id}]]"));
            if !block.link_ids.iter().any(|(name, _)| name == &title) {
                block.link_ids.push((title, id));
            }
        }
    }
}
pub(super) fn display(block: &mut super::Block, workspace: &super::Workspace) {
    for link in links(&block.text).into_iter().rev() {
        let target =
            resolve(&link, block, workspace).map(|d| (d.document.title.clone(), d.id.clone()));
        let title = target
            .as_ref()
            .map(|(title, _)| title.clone())
            .unwrap_or_else(|| link.title.clone());
        if let Some(id) = target.map(|(_, id)| id).or_else(|| link.id.clone()) {
            block.link_ids.retain(|(name, _)| name != &title);
            block.link_ids.push((title.clone(), id));
        }
        replace(block, &link, &format!("[[{title}]]"));
    }
}
fn replace(block: &mut super::Block, link: &DocumentLink, replacement: &str) {
    let chars: Vec<_> = block.text.chars().collect();
    let mut styles = block.character_styles();
    let style = styles.get(link.start).copied().unwrap_or(block.format);
    block.text = chars[..link.start].iter().collect::<String>()
        + replacement
        + &chars[link.end..].iter().collect::<String>();
    styles.splice(
        link.start..link.end,
        std::iter::repeat_n(style, replacement.chars().count()),
    );
    block.set_character_styles(&styles);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn id_links_survive_a_rename() {
        let mut app = crate::FlowriteApp::empty();
        app.title = "Target".into();
        let target = app.workspace.add(app.local_document(), None);
        let mut block = crate::Block::new(4, crate::BlockKind::Text, "é [[Target]]");
        store(&mut block, &app.workspace);
        assert!(block.text.contains(&format!("|{target}]]")));
        app.workspace.rename_document(&target, "Renamed").unwrap();
        display(&mut block, &app.workspace);
        assert_eq!(block.text, "é [[Renamed]]");
        assert_eq!(
            resolve(&links(&block.text)[0], &block, &app.workspace)
                .unwrap()
                .id,
            target
        );
    }
    #[test]
    fn unicode_does_not_shift_document_link_ranges() {
        let parsed = links("é [[Research notes]] and [[Draft]]");
        assert_eq!(parsed[0].start, 2);
        assert_eq!(parsed[0].title, "Research notes");
        assert_eq!(parsed[1].title, "Draft");
        assert!(links("an unfinished [[link").is_empty());
    }
}
