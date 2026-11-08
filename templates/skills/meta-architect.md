---
name: meta-architect
description: Master self-customization skill for scaffolding skills, extensions, roles, and policies.
when_to_load: user asks to create or edit a tool, skill, extension, role, or policy
---

# Meta-Architect Skill

When authoring or modifying A4L components:

1. **Skills:** Put Markdown files with YAML frontmatter in `.a4l/skills/<name>.md`.
2. **Policies:** Put YAML rules in `.a4l/policies/<name>.yaml`.
3. **Profiles:** Put model parameters in `.a4l/profiles/<name>.yaml`.
4. **Validation:** Always validate syntax before finishing.
5. **Human Approval:** Any extension install or policy modification requires explicit human approval.
