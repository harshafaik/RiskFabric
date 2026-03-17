# RiskFabric Documentation Style Guide

This guide defines the standards for all codebase documentation within the RiskFabric project.

## Core Mandates
- **Focus on "Why":** Documentation explains the intent and architectural purpose. The code itself explains the "what."
- **No First Person:** Avoid "I", "we", "my", or "me". Use passive voice or third-person phrasing (e.g., "The system is designed to...", "The engine implements...").
- **Understated Tone:** Use precise, professional, and neutral language. Eliminate promotional, flowery, or superlative phrasing.
- **No Line-by-Line:** Avoid walking through the code line-by-line.
- **Minimal Code Excerpts:** Only include code when it is necessary to illustrate a specific, critical design decision.
- **Length:** Target 150-250 words per component file.

## Document Structure
Every component document must follow this structure:
1. **Summary:** Concise statement of the file's primary responsibility.
2. **Architectural Decisions:** Why the approach was taken and the tradeoffs involved.
3. **System Integration:** How the file interacts with other components (Kafka, Redis, Parquet, etc.).
4. **Known Issues:** Honest and specific technical debt or limitations (e.g., "The population size is hardcoded...").

## Workflow
1. **Understanding:** Present an understanding of the file's role.
2. **Drafting:** Write the `.md` file in `documentation/components/`.
3. **Integration:** Add the new file to `documentation/SUMMARY.md`.
4. **Context Updates:** Update high-level docs to reflect changes or new components.
