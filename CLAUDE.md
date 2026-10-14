Always look for and read one or more skills that are related to the task you are performing.

### Application and Library State

The application and library are in an "alpha" state, meaning they are still under active development and any feature that we change should be fully re-written without worrying about backward compatibility.

### Token Constraints & Workflow Execution

- **Strict Maximum Output:** You are capped at 2,048 tokens per individual API turn/model response.
- **Incremental Steps Required:** Do not attempt massive multi-file rewrites or comprehensive tasks in a single turn. Break all tasks down into small, iterative micro-steps.
- **Check-in Protocol:** Complete one micro-step (e.g., editing a single isolated block of code), verify the output, and wait for confirmation before initiating the next step.
