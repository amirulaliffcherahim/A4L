---
name: create-skill
description: Guide for authoring new A4L markdown skills with frontmatter.
when_to_load: user asks to add, create, or author a skill
---

# Creating an A4L Skill

Skills are hot-reloaded Markdown files located in `.a4l/skills/<name>.md`.

## Frontmatter Requirements
Every skill MUST begin with YAML frontmatter:
```yaml
---
name: my-skill
description: Concise one-line summary of what this skill provides.
when_to_load: Trigger phrase explaining when to load this skill.
---
```

## Structure
1. **Core Procedure:** Bulleted steps describing the capability.
2. **Tools / Commands:** Concrete commands or tool calls to use.
3. **Verification Checklist:** Items to check before declaring done.
4. **Keep it short:** Max ~50 lines to prevent context degradation.
