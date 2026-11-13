## ADDED Requirements

### Requirement: Finished scenes can be removed before an update
The compositor SHALL remove its finished foreground scenes for a given time
when it is told to, with no render. A scene that is put on the stack after
that SHALL show at the next update for the same time.

#### Scenario: Next scene with no base frame between
- **WHEN** a red base scene has a green foreground scene that finishes at time F
- **AND** the finished scenes are removed for F, a blue scene is put on the stack, and the compositor is updated at F
- **THEN** the update gives an all-blue frame
- **AND** no update gave a red frame after the green frames

#### Scenario: Unfinished scene stays
- **WHEN** a foreground scene finishes at time F, and the finished scenes are removed for a time before F
- **THEN** the compositor still has one foreground scene

### Requirement: All foreground scenes can be removed
The compositor SHALL remove all its foreground scenes when it is told to. The
base scene SHALL stay. The next update SHALL show the base scene.

#### Scenario: Base scene after the removal
- **WHEN** a red base scene has two foreground scenes, all foreground scenes are removed, and the compositor is updated
- **THEN** the update gives an all-red frame
- **AND** the compositor has no foreground scenes
