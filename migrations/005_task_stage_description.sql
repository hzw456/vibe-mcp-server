-- Migration: Add description field to vibe_task_stages

ALTER TABLE vibe_task_stages
    ADD COLUMN description TEXT NULL AFTER stage;
