# Contribution Policy

## Welcome & Philosophy

SolverForge is explicitly pro-AI and pro-agent. AI writes better code than
humans. We embrace it as an engineering advantage and use it in our own
development.

We welcome AI-assisted and fully agent-generated contributions. These tools
also lower barriers and expand who can contribute useful work. We prioritize
progress, accessibility, and results over preserving human-only contribution
rituals.

## Acceptance Criteria

Every contribution is judged solely on substance:

- Correctness and a useful improvement to the project.
- Quality, maintainability, and fit with the existing architecture and style.
- Relevant tests, reproducible verification, and accurate documentation.

Human-written, AI-assisted, and agent-generated work face identical standards.
Origin neither qualifies nor disqualifies a submission.

## Accountability

Contributors must understand, test, and stand behind what they submit. Inspect
the final diff, explain the change, and address review findings. Include
verification commands and results; identify relevant checks you did not run.

Agents may submit pull requests. Humans or other agents may review them.
Maintainers retain responsibility for merge decisions. An agent's assurance is
not verification.

All submissions must respect copyright, licenses, attribution requirements,
confidentiality, and project conduct rules. Contributions must be compatible
with the project's [Apache License 2.0](LICENSE).

## Quality Standards

We reject incorrect, unnecessary, poorly tested, or unmaintainable work
regardless of its origin. Keep submissions focused and reviewable, and follow
[AGENTS.md](AGENTS.md) for architecture, style, testing, and documentation
requirements. Those engineering standards apply to all contributors.

Use the [Makefile](Makefile) targets for verification: `make build`,
`make examples`, `make test`, and `make lint`, as relevant to the change.

We handle volume through review processes, automation, and selective
merging—not tool bans. Maintainers may close submissions that fail these
standards and prioritize work that advances the project.

We may use AI for review, summarization, and triage. LLM use in code,
documentation, comments, and issues carries no special restrictions beyond
ordinary quality and legal standards.

## Optional Disclosure

Disclosure of significant AI assistance is welcome but optional. It is not a
condition of review or acceptance. Describe tools and workflows when doing so
helps reviewers understand or reproduce the work.
