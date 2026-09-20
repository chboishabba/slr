CREATE TABLE IF NOT EXISTS digital_esd_screening (
    run TEXT NOT NULL,
    stage TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    record_count INTEGER NOT NULL,
    status TEXT NOT NULL,
    digest TEXT NOT NULL,
    details TEXT,
    PRIMARY KEY (run, stage)
);
