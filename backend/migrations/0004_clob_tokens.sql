-- =============================================================================
-- Migration 0004: Add CLOB Token IDs to Markets Table
-- =============================================================================

-- Add up_token_id and down_token_id columns to store Polymarket 77-digit token IDs
ALTER TABLE markets ADD COLUMN up_token_id TEXT;
ALTER TABLE markets ADD COLUMN down_token_id TEXT;
