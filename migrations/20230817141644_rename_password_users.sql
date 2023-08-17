-- Add migration script here
ALTER table users RENAME password to password_hash;