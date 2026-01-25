---
title: Employee Scheduling (Rust)
emoji: 📅
colorFrom: yellow
colorTo: red
sdk: docker
app_port: 7860
pinned: false
license: apache-2.0
short_description: SolverForge Quickstart for Employee Scheduling in Rust
---

# Employee Scheduling (Rust)

Schedule shifts to employees, accounting for employee availability and shift skill requirements.

- [Prerequisites](#prerequisites)
- [Run the application](#run-the-application)
- [Test the application](#test-the-application)
- [REST API](#rest-api)
- [Constraints](#constraints)
- [More information](#more-information)

## Prerequisites

1. Install [Rust](https://www.rust-lang.org/tools/install) (1.70 or later):

   ```sh
   $ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

## Run the application

1. Git clone the solverforge-quickstarts repo and navigate to this directory:

   ```sh
   $ git clone https://github.com/SolverForge/solverforge-quickstarts.git
   ...
   $ cd solverforge-quickstarts/rust/employee-scheduling
   ```

2. Build and run the application:

   ```sh
   $ cargo run --release
   ```

3. Visit [http://localhost:7860](http://localhost:7860) in your browser.

4. Click on the **Solve** button.

## Test the application

1. Run tests:

   ```sh
   $ cargo test
   ```

## Docker

You can also run the application using Docker:

```bash
# From repository root
$ docker build -f rust/employee-scheduling/Dockerfile -t employee-scheduling-rust .
$ docker run -p 7860:7860 employee-scheduling-rust
```

Then visit [http://localhost:7860](http://localhost:7860) in your browser.

## REST API

- `GET /demo-data` - List available demo datasets
- `GET /demo-data/{id}` - Get specific demo data
- `POST /schedules` - Start solving (returns job ID)
- `GET /schedules/{id}` - Get current solution
- `DELETE /schedules/{id}` - Stop solving
- `PUT /schedules/analyze` - Analyze constraint violations

## Constraints

**Hard Constraints** (must be satisfied):
- Required skill match
- No overlapping shifts
- Minimum 10 hours between shifts
- One shift per day per employee
- Respect unavailable dates

**Soft Constraints** (optimized):
- Avoid undesired dates
- Prefer desired dates
- Balance shift assignments across employees

## More information

Visit [solverforge.org](https://www.solverforge.org).
