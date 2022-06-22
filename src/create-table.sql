CREATE TABLE IF NOT EXISTS thermometer_history (
    time                TEXT NOT NULL,
    name                TEXT NOT NULL,
    status              TEXT CHECK(status in ('Connected', 'Disconnected')),
    last_measurement    REAL,
    target_temperature  REAL
);