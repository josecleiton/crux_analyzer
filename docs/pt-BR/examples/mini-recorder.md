# Mini Recorder

## Núcleo: MiniRecorder

### Máquina: RecorderState

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

| De | Evento | Para | Efeitos |
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

### Máquina: UploadState

```mermaid
stateDiagram-v2
    Empty --> Uploading: StopPressed
    Uploading --> Synced: UploadFinished
```

| De | Evento | Para | Efeitos |
| --- | --- | --- | --- |
| Empty | `StopPressed` | Uploading | — |
| Uploading | `UploadFinished` | Synced | — |
