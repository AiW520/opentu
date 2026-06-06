## ADDED Requirements

### Requirement: Batch canvas insertion uses viewport flow layout

Batch canvas insertion SHALL place inserted items left-to-right using the current canvas viewport width in canvas coordinates, then wrap to a new row when the next item would exceed that width.

For media items inserted through the batch path, the final insertion size SHALL remain stable for the batch flow and SHALL NOT be rewritten after load in a way that changes the batch spacing.

#### Scenario: Wide viewport fits multiple items on one row
- **GIVEN** the current canvas viewport can fit multiple batch items at the active zoom
- **WHEN** a batch insertion runs
- **THEN** items SHALL be inserted left-to-right on the same row until the next item would exceed the viewport width

#### Scenario: Narrow or zoomed-in viewport wraps items
- **GIVEN** the current canvas viewport cannot fit the next batch item at the active zoom
- **WHEN** a batch insertion runs
- **THEN** the next item SHALL be inserted at the batch start x coordinate on the next row
- **AND** the next row SHALL start after the tallest item in the previous row plus the configured vertical gap

#### Scenario: Batch insertion scrolls to the batch center
- **GIVEN** one or more items were inserted by a batch insertion
- **WHEN** insertion completes
- **THEN** the canvas SHALL scroll toward the center of the inserted batch bounds

#### Scenario: Batch media sizes stay stable
- **GIVEN** image or video items are inserted through the batch flow
- **WHEN** the media finishes loading
- **THEN** the batch spacing SHALL remain based on the original batch layout size
- **AND** later media resizing SHALL NOT shift the row spacing for the completed batch
