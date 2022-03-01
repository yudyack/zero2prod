-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES (
    '9c3f0561-a949-4ee0-b1de-26b34161201b',
    'admin',
    '$argon2id$v=19$m=15000,t=2,p=1$LHZZ5Q3cujlLm8/ontmu8g$uxL2wAdo0Z6RWbjTQpe9QbizSaHwikUUt7BFWTsBsZc'
    -- testuserpassword
)
