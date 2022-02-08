-- Add Subscription Tokens table
CREATE TABLE subscription_token (
    subscription_token TEXT NOT NULL,
    subscriber_id uuid NOT NULL
        REFERENCES subscriptions(id),
    PRIMARY KEY (subscription_token)
);