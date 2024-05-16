# task-tracker-rs
Restfull API backend to manage status of long running tasks.
## Description
task-tracker-rs is a backend service that manage only the status, not the tasks themselves.  
A service can use it to create a task, update the progress and status of completion. An id to track the task will be given at creation, allowing to view or update it.
## Features

- no database, run in memory
- timelapse to forget finished tasks.
- pulled and pushed status
- non opiniated about the type of tasks.
- useable by multiples services at the same time.
- separation of privileges using secret keys and tokens.

## Licence

This software is GPL 3.
