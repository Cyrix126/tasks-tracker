# OBJECTIVES of task-tracker
## Serve a task manager API
The purpose is to serve an API to manage status of long running tasks.
### CRUD
create a task, returns:  
- an id to retrieve it.
- a token to update the progress and status
- a token to abort the task
- a token to view the progress and status  

read task status  

update a task status  

delete task  

### Admin operations
list all tasks
### Explanation
The API must serves endpoints to allow backend service to create tasks, update the progress and completion status.  
The endpoint creating a task must return an id to view/manage the task.  
The API must also serve endpoint to view current tasks, view a specific task and to cancel it.  
The status of a task can be pulled (client need to ask) or pushed (client listen and receive status).  
## push notification for status
When creating a task, a listening address can be given for push notification when status is updated.  
For example, if a user abort the task, the server can receive a notification and cancel it.
## No awareness of the actual process
The progress and completion status of a task are solely modified through the API that can be used by the client or backend service. long-task-manager does not have any view/control of the process of the task it is representing. It is the role of the service creating the task to update the status of it.
## Optimized
Tasks are stored in memory and forgotten after a timelapse provided by the backend service.
API can use bincode format instead of json. (bitcode later when https://github.com/SoftbearStudios/bitcode/issues/24#issuecomment-2021655274 is resolved.) 
## Operations on tasks separated by tokens
Different actions can be used by different types of third parties. Tokens for different actions are issued when creating a task.  
A secret "admin" key for managing all the task can be configured without knowing their own token.
