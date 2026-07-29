# Mini Recorder

## Core: MiniRecorder

### Machine: RecorderState

Where one recording session lives, from arming the microphone to a
finished upload.

```mermaid
stateDiagram-v2
    Idle --> Recording: RecordPressed
    Recording --> Paused: PausePressed
    Paused --> Recording: ResumePressed
    Recording --> Uploading: StopPressed
    Paused --> Uploading: StopPressed
    Uploading --> Completed: UploadFinished
    Failed --> Uploading: RetryPressed
    Recording --> Failed: Failed
    Paused --> Failed: Failed
    Uploading --> Failed: Failed
    note right of Idle : Nothing is being recorded yet. Every session starts and ends here.
    note right of Recording : Capturing audio from the microphone.
    note right of Paused : Capture is suspended and the buffer is kept.
    note right of Uploading : The finished take is on its way to the server.
    note right of Completed : The take is stored and the session is done.
    note right of Failed : The upload gave up. The session is kept so it can be sent again.
```

#### States

| State | Description | Markers | Tags |
| --- | --- | --- | --- |
| Idle | Nothing is being recorded yet. Every session starts and ends here. | — | — |
| Recording | Capturing audio from the microphone. | — | — |
| Paused | Capture is suspended and the buffer is kept. | — | — |
| Uploading | The finished take is on its way to the server. | — | — |
| Completed | The take is stored and the session is done. | — | — |
| Failed | The upload gave up. The session is kept so it can be sent again. | failure | `retryable` |

##### Paused

Capture is suspended and the buffer is kept.

`by_system` distinguishes a pause the user asked for from one an
interruption forced.

| From | Event | To | Effects |
| --- | --- | --- | --- |
| Idle | `RecordPressed` | Recording | `AudioOperation::Start` |
| Recording | `PausePressed` | Paused | — |
| Paused | `ResumePressed` | Recording | — |
| Recording | `StopPressed` | Uploading | `AudioOperation::Stop` |
| Paused | `StopPressed` | Uploading | `AudioOperation::Stop` |
| Uploading | `UploadFinished` | Completed | — |
| Failed | `RetryPressed` | Uploading | — |
| Recording | `Failed` | Failed | — |
| Paused | `Failed` | Failed | — |
| Uploading | `Failed` | Failed | — |

### Machine: UploadState

Mirrors the finished take to the server.

Being folded into `RecorderState`, which already tracks the upload.

**Markers:** deprecated

```mermaid
stateDiagram-v2
    Empty --> Uploading: StopPressed
    Uploading --> Synced: UploadFinished
```

| From | Event | To | Effects |
| --- | --- | --- | --- |
| Empty | `StopPressed` | Uploading | — |
| Uploading | `UploadFinished` | Synced | — |

### Events

| Event | Description |
| --- | --- |
| `RecordPressed` | The user hit the record button on the main screen. |
| `RetryPressed` | Retry the failed upload, keeping the recorded take. |

### Effects

| Effect | Description |
| --- | --- |
| `AudioOperation::Start` | Arms the microphone and begins capturing into the session buffer. |
