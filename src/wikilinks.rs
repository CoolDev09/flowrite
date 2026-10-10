#[derive(Debug, Clone, PartialEq)]
pub(super) struct DocumentLink {
    pub title: String,
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
        let title = text[start + 2..end - 2].trim();
        if !title.is_empty() {
            found.push(DocumentLink {
                title: title.to_owned(),
                start: text[..start].chars().count(),
                end: text[..end].chars().count(),
            });
        }
        offset = end;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_does_not_shift_document_link_ranges() {
        let parsed = links("é [[Research notes]] and [[Draft]]");
        assert_eq!(parsed[0].start, 2);
        assert_eq!(parsed[0].title, "Research notes");
        assert_eq!(parsed[1].title, "Draft");
        assert!(links("an unfinished [[link").is_empty());
    }
}
