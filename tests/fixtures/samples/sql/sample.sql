-- v0.4.0 fixture：用于 preprocess_6_sources e2e 验证 SqlReader 读取。
-- 多语句类型混合（select / insert / update / create / drop），含分号在字符串内的反例。
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  username TEXT NOT NULL,
  email TEXT
);
INSERT INTO users (id, username, email) VALUES (1, 'alice', 'alice@example.com');
INSERT INTO users (id, username, email) VALUES (2, 'bob; jr', 'bob@example.com');
SELECT id, username, email FROM users WHERE username = 'a;b';
UPDATE users SET email = 'new@example.com' WHERE id = 1;
DROP TABLE users;
