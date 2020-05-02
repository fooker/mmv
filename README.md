`mmv` - Linux mass file rename / mover

mmv is a utility to move huge masses of files while selectively renaming and deleting these.
It leverages the power of `vim` allowing to use all your editor kung-fu for file renaming.

## Concept
mmv uses a stateful workspace concept.
It keeps track of the files in the workspace and the assigned actions.
The user can edit the assigned action multiple times and inspect the result before execution the set of changes in the workspace.

The list of all known files in the workspace is stored in a file inside the workspace line-by-line (the sources file).
The actions assigned to these files are stored in a second file which can be edited by the user (the targets file).
The lines in these files match up to define the change set.
I.e. the first line in the source file and the first line in the target file will define the first change - and so on.
Therefore the number of lines mus be equal in both files to define a valid change set.
If the change set is not valid, it can not be updated or executed.

## Action Format
The target file contins the actions line by line.
Each action must be one of the follow three types:

### Move
The file is moved to this location.
The location must be a valid file name and will be interpreted as path relative to the target given during execution.

### Delete
The file will be deleted.
An empty line (containing only zero or more whitespaces) will mark the file for deletion

### Ignore
The file will be ignored.
A line must start wih one or more whitespaces.
The remaining line will be used as comment.


## Workflow

* Init the mmv workspace in the current directory.
  ```
  mmv init
  ```

* Open the editor to modify actions 
  ```
  mmv edit
  ```

* Inspect the workspace status
  ```
  mmv status
  ``` 

* Edit again
  ```
  mmv edit
  ```
  
* Exit the changes
  ```
  mmv execute
  ```
 
