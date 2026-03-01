use comrak::arena_tree::NodeEdge;
use comrak::nodes::NodeValue;
use comrak::options::Extension;
use comrak::{Arena, Options};
use std::sync::LazyLock;

// This configuration follows Discord's version of Markdown.
static OPTIONS: LazyLock<Options> = LazyLock::new(|| Options {
    extension: Extension::builder()
        .strikethrough(true)
        .underline(true)
        .subscript(true)
        .subtext(true)
        .spoiler(true)
        .build(),
    ..Default::default()
});

#[must_use]
pub fn strip_markdown(text: &str) -> String {
    let arena = Arena::new();
    let root = comrak::parse_document(&arena, text, &OPTIONS);

    let mut output = String::with_capacity(text.len());
    for edge in root.traverse() {
        if let NodeEdge::Start(node) = edge {
            match &node.data().value {
                NodeValue::Text(text) => {
                    output.push_str(text);
                }
                NodeValue::SoftBreak => {
                    output.push('\n');
                }
                _ => {}
            };
        } else if let NodeEdge::End(node) = edge {
            match &node.data().value {
                NodeValue::Item(..) => {
                    output.push('\n');
                }
                _ => {}
            };
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use crate::markdown::strip_markdown;
    use pretty_assertions::assert_str_eq;

    // Based from: https://support.discord.com/hc/en-us/articles/210298617-Markdown-Text-101-Chat-Formatting-Bold-Italic-Underline
    static CASES: &[(&str, &str)] = &[
        ("Hello **World!**", "Hello World!"),
        ("Hello, *worl[d](https://example.com/)*", "Hello, world"),
        ("*Hello*", "Hello"),
        ("***bold***", "bold"),
        ("__*very deeep*__", "very deeep"),
        ("# Big Header", "Big Header"),
        ("-# subtexts", "subtexts"),
        ("- code\n- sleep\n- cry", "code\nsleep\ncry\n"),
        ("> pink\n> blue", "pink\nblue"),
        (">>> wow\nvery amazing right?", "wow\nvery amazing right?"),
        // this should retain its original content
        (
            "Hello World\nI like this world.",
            "Hello World\nI like this world.",
        ),
        // code blocks are stripped in this case here
        ("`simple code`", ""),
        ("```\nLONG CODE\n```", ""),
    ];

    #[test]
    fn should_strip_markdown() {
        for (text, expected) in CASES {
            let output = strip_markdown(text);
            assert_str_eq!(expected, &output, "should strip markdown as expected");
        }
    }
}
