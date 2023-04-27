-- migrations/{timestamp}_create_subscription_tokens_table.sql
-- Create Subscriptions Token Table

CREATE TABLE subscription_tokens(
   subscription_token TEXT NOT NULL UNIQUE,
   subscription_id uuid NOT NULL REFERENCES subscriptions (id),
   PRIMARY KEY (subscription_token)
);