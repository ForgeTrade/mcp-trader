# MCP Trader Constitution

<!--
SYNC IMPACT REPORT (2025-10-18)
═══════════════════════════════════════════════════════════════════════════════
Version: 1.0.0 (Initial creation)

Changes:
- Initial constitution creation for mcp-trader project
- 7 core principles established based on user requirements
- Governance and compliance framework established

Principles Defined:
1. Simplicity and Readability
2. Library-First Development
3. Justified Abstractions
4. DRY Principle
5. Service and Repository Patterns
6. 12-Factor Methodology
7. Minimal Object-Oriented Programming

Templates Status:
✅ plan-template.md - Reviewed, compatible with constitution
✅ spec-template.md - Reviewed, compatible with constitution
✅ tasks-template.md - Reviewed, compatible with constitution
✅ checklist-template.md - Not modified (checklist generation tool)
✅ agent-file-template.md - Not modified (agent template)

Follow-up TODOs:
- None

═══════════════════════════════════════════════════════════════════════════════
-->

## Core Principles

### I. Simplicity and Readability

Code MUST be simple, clear, and readable. Every piece of code must be understandable by team members without extensive documentation or mental overhead.

**Rules**:
- Variables, functions, and types MUST have descriptive, clear names following language conventions
- Complex logic MUST be broken down into smaller, well-named functions
- Code MUST follow language-specific best practices and style guides
- Nested complexity (deep indentation, multiple nested conditions) is discouraged
- Comments are required only when the "why" is not obvious from the code itself

**Rationale**: Simple, readable code reduces bugs, accelerates onboarding, and enables faster iteration. Complexity compounds maintenance costs exponentially.

---

### II. Library-First Development

Before implementing functionality from scratch, developers MUST search for and evaluate existing libraries and tools that solve the problem.

**Rules**:
- For any non-trivial functionality, research MUST be conducted to find existing solutions
- Custom implementations are justified only when:
  - No suitable library exists
  - Existing libraries have deal-breaking limitations (performance, security, licensing)
  - The dependency overhead outweighs the implementation cost
- When using external libraries, prefer well-maintained, popular solutions with active communities

**Rationale**: Leveraging existing, battle-tested libraries reduces development time, minimizes bugs, and ensures we build on proven solutions rather than reinventing the wheel.

---

### III. Justified Abstractions

Abstractions in code MUST be justified by concrete needs, not speculative future requirements.

**Rules**:
- Do not introduce abstractions (interfaces, base classes, generic patterns) until they solve an actual, present problem
- Every abstraction MUST have a clear purpose documented in code or design docs
- Follow YAGNI (You Aren't Gonna Need It) - build what is needed now, not what might be needed later
- If an abstraction is added speculatively, it MUST be explicitly justified in code review

**Rationale**: Premature abstraction creates unnecessary complexity, makes code harder to understand, and often leads to over-engineered solutions that don't match actual requirements when they emerge.

---

### IV. DRY Principle (Don't Repeat Yourself)

Code duplication MUST be eliminated when the same logic appears 2-3+ times.

**Rules**:
- When similar code appears twice, note it but duplication may remain
- When identical or very similar code appears 3+ times, refactor into a shared function, method, or module
- Extracted code MUST have a clear, descriptive name
- The DRY principle applies to logic, not to incidental similarity (e.g., similar-looking but semantically different code)

**Rationale**: Duplication increases maintenance burden and bug surface area. Changes must be replicated across multiple locations, increasing the risk of inconsistency and errors.

---

### V. Service and Repository Patterns

When working with data persistence and business logic, use the Service and Repository patterns to maintain separation of concerns.

**Rules**:
- **Repository pattern**: Data access logic MUST be isolated in repository modules/classes
  - Repositories handle database queries, ORM interactions, and data mapping
  - Repositories return domain entities, not database-specific objects
- **Service pattern**: Business logic MUST be encapsulated in service modules/classes
  - Services orchestrate repositories and implement business rules
  - Services expose a clean API to application layers (CLI, API endpoints, etc.)
- Use these patterns when:
  - The application has data persistence requirements
  - Business logic complexity justifies separation
  - Testing and maintainability benefit from isolation

**Rationale**: These patterns provide clear boundaries between data access, business logic, and presentation, making the codebase more testable, maintainable, and easier to reason about.

---

### VI. 12-Factor Methodology

All applications MUST follow the principles of the 12-Factor methodology to ensure scalability, portability, and operational excellence.

**Key requirements**:
1. **Codebase**: One codebase tracked in version control
2. **Dependencies**: Explicitly declare and isolate dependencies
3. **Config**: Store configuration in environment variables, NEVER in code
4. **Backing services**: Treat backing services (databases, message queues) as attached resources
5. **Build, release, run**: Strictly separate build and run stages
6. **Processes**: Execute the app as stateless processes
7. **Port binding**: Export services via port binding
8. **Concurrency**: Scale out via the process model
9. **Disposability**: Fast startup and graceful shutdown
10. **Dev/prod parity**: Keep development, staging, and production as similar as possible
11. **Logs**: Treat logs as event streams (write to stdout/stderr)
12. **Admin processes**: Run admin/management tasks as one-off processes

**Rationale**: The 12-Factor methodology ensures applications are cloud-native, portable, and maintainable in modern deployment environments.

---

### VII. Minimal Object-Oriented Programming

Object-Oriented Programming (OOP) SHOULD be used sparingly and only when clearly justified.

**Rules**:
- Prefer procedural or functional programming styles when appropriate
- Use simple functions, modules, and data structures as the default approach
- OOP (classes, inheritance, polymorphism) is justified when:
  - You need to model domain entities with both data and behavior
  - Polymorphism provides clear value (e.g., plugin systems, strategy patterns)
  - Encapsulation significantly improves code organization
- Avoid deep inheritance hierarchies (prefer composition over inheritance)
- Avoid excessive use of design patterns for their own sake

**Rationale**: OOP can introduce unnecessary complexity, indirection, and cognitive overhead. Many problems are better solved with simpler procedural or functional approaches. Use OOP as a tool when it adds value, not as a default paradigm.

---

## Development Workflow

### Code Review Requirements

All code changes MUST pass code review with explicit verification of:
- Compliance with all seven core principles
- Justified abstractions (if any are introduced)
- Proper use of external libraries (Library-First principle)
- DRY principle adherence (no unjustified duplication)
- Service/Repository pattern usage (if applicable)
- 12-Factor compliance (configuration, logging, statelessness)
- Minimal OOP usage (no unnecessary classes or inheritance)

### Complexity Justification

Any deviation from the constitution MUST be documented and justified in:
- Pull request descriptions
- Code comments (for local deviations)
- The project's `plan.md` Complexity Tracking section (for architectural decisions)

### Testing and Quality Gates

- Tests SHOULD be written for business logic in services
- Integration tests SHOULD cover repository contracts
- Tests MUST NOT introduce complexity that violates core principles
- All tests MUST pass before merging

---

## Governance

### Amendment Process

This constitution can be amended through:
1. Proposal of changes via issue or discussion
2. Team consensus (or project owner approval for personal projects)
3. Documentation of the rationale for changes
4. Update of the constitution version according to semantic versioning

### Versioning Policy

Version follows **MAJOR.MINOR.PATCH** format:
- **MAJOR**: Backward-incompatible changes (e.g., removing or redefining principles)
- **MINOR**: New principles or sections added
- **PATCH**: Clarifications, wording improvements, non-semantic changes

### Compliance Review

All major features and architectural decisions MUST undergo a constitution compliance review before implementation begins. This review MUST be documented in the feature's `plan.md` file under the "Constitution Check" section.

### Template Synchronization

When the constitution is amended, all dependent templates MUST be reviewed and updated:
- `.specify/templates/plan-template.md` - Update Constitution Check section
- `.specify/templates/spec-template.md` - Ensure requirements align with principles
- `.specify/templates/tasks-template.md` - Ensure task structure reflects principles

---

**Version**: 1.0.0 | **Ratified**: 2025-10-18 | **Last Amended**: 2025-10-18
