# Task Manager Concurrency

`TaskManager` remains intentionally serialized for now.

The safe expansion point is to classify requests as:

- **Queries**: bounded concurrent read-only requests such as list and inspect.
- **Operations**: one mutating request at a time, after policy confirmation.
- **Streams**: one cancellable stream per resource, with bounded event delivery.

The current manager has one active `JoinHandle`, which guarantees that mutations cannot overlap and that `x` cancellation and shutdown cancellation cover the only running task. Replacing it with a registry would require per-task cancellation tokens, independent completion cleanup, and explicit limits for query and stream classes. That change is deferred until those guarantees have dedicated tests.

No concurrency regression was introduced: task serialization, bounded event channels, safety-policy dispatch, and cancellation remain unchanged.
