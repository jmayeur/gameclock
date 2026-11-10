# Legacy behavior (reference)

What the old Python/Node clock does, collected from `~/dev/gameclock`
(`main/`, `v2/`, `backup/`). This is input for OpenSpec proposals, not a spec:
each behavior is re-decided when its change is proposed. Where `main/` and `v2/`
differ, both are noted. The Pi runs `.../bindings/python/samples/clock.py`, and
which variant that file is hasn't been confirmed.

## Rendering

- 64×32 panel. Text is drawn with BDF fonts: `7x13B` for the clock and
  messages, `texgyre-27` for the bedtime alert.
- Clock format: `%I:%M%p` (for example `09:05PM`), drawn at x=8, baseline y=20.
  It redraws only when the minute or the color changes.

## Time-of-day schedule (hardcoded)

From `v2/clock.py`:

| Hours | Clock color | Brightness |
|---|---|---|
| 00:00–05:59 | red `(255,0,0)` | 25 |
| 06:00–09:59 | blue `(0,0,255)` | 100 |
| 10:00–20:59 | blue | 60 |
| 21:00–22:59 | mellow `(255,140,0)` | 60 |
| 23:00–23:14 | mellow | 100 on bedtime nights, otherwise 25 |
| 23:15–23:59 | mellow | 25 |

**Bedtime nights** are every day except Friday and Saturday (Python
`weekday() not in (4, 5)`).

An earlier version (`backup/`) used blue 07–18, mellow 19–21, red otherwise.
The hours were tuned by editing code, which suggests they should be
configurable.

## Bedtime warning (bedtime nights, 23:00–23:15)

- `v2`: during minutes 0–4, flashes `BED!!!!!` in red when `second % 10 == 0`,
  otherwise shows the clock.
- `main`: same flash during minutes 0–4. During minutes 5–15, triggers the
  scrolling bedtime alert every 10 seconds.

## Commands

The web form writes `msg=<value>` to a semaphore file
(`.../samples/semaphore/msg.txt`). The renderer polls it, reads it, and
truncates it.

| Value | Behavior |
|---|---|
| `panic88` | Full-screen color-wheel fill, cycling red→green→blue, with `PANIC!!` in black or white for contrast. About 20 s in `v2`. `main` uses 4000 × 5 ms iterations. |
| `bedtime` | `GAME OVER - TURN IT OFF!` scrolls in lime `(50,205,50)` using `texgyre-27` at y=25. 2 passes (`v2`) or 1 (`main`). |
| `brightness` | Scrolls the current brightness number. |
| any other text | Scrolls in green `(0,150,0)` at y=20, with `+` turned into a space. 4 passes (`v2`) or 5 (`main`). The `v2` server trims whitespace and caps messages at 64 chars. |

Scrolling moves 1 px per frame, every 30–50 ms.

## Web control (Node)

- `v2/server.js` (Node, no dependencies) has `GET /` for the form,
  `POST /message`, and `GET /health`. The form has buttons for **Time for Bed**
  and **PANIC**, plus a free-text field.
- The Pi runs Node 10.24. Node no longer ships official builds for ARMv6.

## Abandoned or partial features

- **Admin settings** (`backup/admin.html`, `getConfigs`): shutdown-warning time
  (9:15 PM), bedtime-alert time (9:30 PM), force-dim, and a color override,
  stored as JSON. The renderer side has bugs and was never finished.
- **Calendar/events** (`main/server.js`): `POST /api/events` stored
  `events.json`, sent an `evntsync` command, and pushed updates over a
  WebSocket on :8080. The renderer side was removed in `v2`.
- `drawCircle` exists in the renderer but is never used.
