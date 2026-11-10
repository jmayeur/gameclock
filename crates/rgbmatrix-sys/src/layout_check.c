/*
 * Build-time check that led-matrix-c.h still has the struct layout our
 * hand-written Rust bindings mirror (docs/adr/0009). build.rs passes the
 * expected sizes and offsets as LAYOUT_* defines from one table; src/ffi.rs
 * asserts the Rust structs against the same table. A mismatch fails the build.
 */
#include <stddef.h>
#include "led-matrix-c.h"

#define CHECK(expr, expected) \
    _Static_assert((expr) == (expected), #expr " != " #expected)

CHECK(sizeof(struct Color), LAYOUT_COLOR_SIZE);

CHECK(sizeof(struct RGBLedMatrixOptions), LAYOUT_OPTIONS_SIZE);
CHECK(offsetof(struct RGBLedMatrixOptions, hardware_mapping), LAYOUT_OPTIONS_HARDWARE_MAPPING);
CHECK(offsetof(struct RGBLedMatrixOptions, rows), LAYOUT_OPTIONS_ROWS);
CHECK(offsetof(struct RGBLedMatrixOptions, cols), LAYOUT_OPTIONS_COLS);
CHECK(offsetof(struct RGBLedMatrixOptions, chain_length), LAYOUT_OPTIONS_CHAIN_LENGTH);
CHECK(offsetof(struct RGBLedMatrixOptions, parallel), LAYOUT_OPTIONS_PARALLEL);
CHECK(offsetof(struct RGBLedMatrixOptions, pwm_bits), LAYOUT_OPTIONS_PWM_BITS);
CHECK(offsetof(struct RGBLedMatrixOptions, pwm_lsb_nanoseconds), LAYOUT_OPTIONS_PWM_LSB_NANOSECONDS);
CHECK(offsetof(struct RGBLedMatrixOptions, pwm_dither_bits), LAYOUT_OPTIONS_PWM_DITHER_BITS);
CHECK(offsetof(struct RGBLedMatrixOptions, brightness), LAYOUT_OPTIONS_BRIGHTNESS);
CHECK(offsetof(struct RGBLedMatrixOptions, scan_mode), LAYOUT_OPTIONS_SCAN_MODE);
CHECK(offsetof(struct RGBLedMatrixOptions, row_address_type), LAYOUT_OPTIONS_ROW_ADDRESS_TYPE);
CHECK(offsetof(struct RGBLedMatrixOptions, multiplexing), LAYOUT_OPTIONS_MULTIPLEXING);
CHECK(offsetof(struct RGBLedMatrixOptions, disable_hardware_pulsing), LAYOUT_OPTIONS_DISABLE_HARDWARE_PULSING);
CHECK(offsetof(struct RGBLedMatrixOptions, show_refresh_rate), LAYOUT_OPTIONS_SHOW_REFRESH_RATE);
CHECK(offsetof(struct RGBLedMatrixOptions, inverse_colors), LAYOUT_OPTIONS_INVERSE_COLORS);
CHECK(offsetof(struct RGBLedMatrixOptions, led_rgb_sequence), LAYOUT_OPTIONS_LED_RGB_SEQUENCE);
CHECK(offsetof(struct RGBLedMatrixOptions, pixel_mapper_config), LAYOUT_OPTIONS_PIXEL_MAPPER_CONFIG);
CHECK(offsetof(struct RGBLedMatrixOptions, panel_type), LAYOUT_OPTIONS_PANEL_TYPE);
CHECK(offsetof(struct RGBLedMatrixOptions, limit_refresh_rate_hz), LAYOUT_OPTIONS_LIMIT_REFRESH_RATE_HZ);
CHECK(offsetof(struct RGBLedMatrixOptions, disable_busy_waiting), LAYOUT_OPTIONS_DISABLE_BUSY_WAITING);
