# API of task-tracker

long-task-manager (LTM) scenario of usage:

client need to execute an action on the server.
The server will create a new task with auth (path protected by reverse proxy).
The server will receive id and token for update and token for read and token for aborting the task.
It will use the update token to update the progress of the task and the status.
The server will give back the url/id and view token to the client.
The client will use the abort token to cancel the task.
LTM sends a notification to the server to an update to the status.
The server will see that the status is aborted and stop the task.
