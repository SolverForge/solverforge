# Extend the Domain

Use the scaffold as a thin starter, then model the real problem in your app.

## What belongs in the domain

- Add entities, problem facts, and planning variables for the real data shape.
- Use field metadata to model standard variables and one or more independent
  list owners in the same project when needed.
- Add derived fields, validation helpers, and sample data beside the domain
  model, not in the scaffold templates.

## When the scaffold is no longer enough

- Create app-specific modules for larger domain logic.
- Move example constraints and sample fixtures into the app once they stop being
  representative of the starter project.
- Keep the generated scaffold thin so it stays a starter, not the source of
  truth.

## Practical path

1. Keep the scaffolded project.
2. Add your real entities, facts, and variable declarations.
3. Replace example data and example constraints with production domain logic.
4. Split large domain code into app modules as it grows.
