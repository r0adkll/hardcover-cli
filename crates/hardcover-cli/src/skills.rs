//! Skills shipped inside the binary so the installed copy always matches the CLI version.
use serde::Serialize;

pub struct Skill {
    pub name: &'static str,
    pub body: &'static str,
}

pub const SKILLS: &[Skill] = &[
    Skill {
        name: "hardcover",
        body: include_str!("../skills/hardcover/SKILL.md"),
    },
    Skill {
        name: "reading-log",
        body: include_str!("../skills/reading-log/SKILL.md"),
    },
    Skill {
        name: "book-research",
        body: include_str!("../skills/book-research/SKILL.md"),
    },
];

#[derive(Serialize)]
pub struct SkillInfo {
    pub name: &'static str,
    pub description: String,
}

impl Skill {
    /// `description:` from the SKILL.md frontmatter.
    pub fn description(&self) -> String {
        frontmatter(self.body)
            .lines()
            .find_map(|l| l.strip_prefix("description:"))
            .map(|d| d.trim().to_string())
            .unwrap_or_default()
    }

    /// Markdown body without the frontmatter block.
    pub fn markdown(&self) -> &'static str {
        let rest = self.body.strip_prefix("---\n").unwrap_or(self.body);
        match rest.find("\n---\n") {
            Some(i) => rest[i + 5..].trim_start_matches('\n'),
            None => self.body,
        }
    }

    /// Cursor rule (`.mdc`): same body, Cursor's frontmatter.
    pub fn as_cursor_rule(&self) -> String {
        format!(
            "---\ndescription: {}\nglobs:\nalwaysApply: false\n---\n\n{}",
            self.description(),
            self.markdown()
        )
    }

    pub fn info(&self) -> SkillInfo {
        SkillInfo {
            name: self.name,
            description: self.description(),
        }
    }
}

fn frontmatter(body: &str) -> &str {
    let rest = body.strip_prefix("---\n").unwrap_or("");
    rest.find("\n---").map(|i| &rest[..i]).unwrap_or("")
}
