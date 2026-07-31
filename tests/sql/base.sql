-- RuT0DataKit base.sql: PII 样本，覆盖 CREATE/INSERT/SELECT/UPDATE 语句类型。
-- 列 schema 与 base.csv 一致（12 列 PII），供 SqlReader 解析多语句验证。

CREATE TABLE IF NOT EXISTS users (
    username   TEXT,
    name       TEXT,
    sex        TEXT,
    birth      TEXT,
    idcard     TEXT,
    phone      TEXT,
    email      TEXT,
    bankcard   TEXT,
    ip         TEXT,
    mac        TEXT,
    address    TEXT,
    password   TEXT
);

INSERT INTO users VALUES ('n0tr00t','张三','男','19900515','110101199005151234','13812345678','zhangsan@example.com','6225887654321098','192.168.1.1','00:1A:2B:3C:4D:5E','广东省深圳市南山区科技路1号','Pass1234');
INSERT INTO users VALUES ('wh1tej0ker','李四','女','19920720','44030419920720123X','13987654321','lisi@example.org','6225881234567890','10.0.0.55','AA:BB:CC:DD:EE:FF','北京市海淀区中关村大街5号','Sec5678');

SELECT username, phone, email FROM users WHERE sex = '男';

UPDATE users SET password = 'NewPass90' WHERE username = 'n0tr00t';
