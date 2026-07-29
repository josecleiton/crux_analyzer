# Mini Recorder

## Core: MiniRecorder

### Machine: RecorderState

```mermaid
stateDiagram-v2
    Idle --> Recording: RecordPressed
    Recording --> Paused: PausePressed
    Paused --> Recording: ResumePressed
    Recording --> Uploading: StopPressed
    Paused --> Uploading: StopPressed
    Uploading --> Completed: UploadFinished
    Recording --> Idle: Failed
    Paused --> Idle: Failed
    Uploading --> Idle: Failed
```

| From | Event | To | Effects |
| --- | --- | --- | --- |
| Idle | `RecordPressed` | Recording | — |
| Recording | `PausePressed` | Paused | — |
| Paused | `ResumePressed` | Recording | — |
| Recording | `StopPressed` | Uploading | — |
| Paused | `StopPressed` | Uploading | — |
| Uploading | `UploadFinished` | Completed | — |
| Recording | `Failed` | Idle | — |
| Paused | `Failed` | Idle | — |
| Uploading | `Failed` | Idle | — |

### Machine: UploadState

```mermaid
stateDiagram-v2
    Empty --> Uploading: StopPressed
    Uploading --> Synced: UploadFinished
```

| From | Event | To | Effects |
| --- | --- | --- | --- |
| Empty | `StopPressed` | Uploading | — |
| Uploading | `UploadFinished` | Synced | — |
