-- Add username to devices (client sysinfo uploads it)
ALTER TABLE devices ADD COLUMN username TEXT NOT NULL DEFAULT '';
