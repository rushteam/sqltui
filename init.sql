-- 初始化测试数据库脚本
-- 创建一些测试表来演示 MySQL 命令行客户端的功能

USE testdb;

-- 创建用户表
CREATE TABLE users (
    id INT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(100) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    first_name VARCHAR(50),
    last_name VARCHAR(50),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_username (username),
    INDEX idx_email (email),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB COMMENT='用户表';

-- 创建文章表
CREATE TABLE posts (
    id INT AUTO_INCREMENT PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    content TEXT,
    author_id INT NOT NULL,
    status ENUM('draft', 'published', 'archived') DEFAULT 'draft',
    view_count INT DEFAULT 0,
    published_at TIMESTAMP NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_author_id (author_id),
    INDEX idx_status (status),
    INDEX idx_published_at (published_at),
    FULLTEXT(title, content)
) ENGINE=InnoDB COMMENT='文章表';

-- 创建标签表
CREATE TABLE tags (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    description TEXT,
    color VARCHAR(7) DEFAULT '#007bff',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_name (name)
) ENGINE=InnoDB COMMENT='标签表';

-- 创建文章标签关联表
CREATE TABLE post_tags (
    post_id INT NOT NULL,
    tag_id INT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (post_id, tag_id),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
) ENGINE=InnoDB COMMENT='文章标签关联表';

-- 创建评论表
CREATE TABLE comments (
    id INT AUTO_INCREMENT PRIMARY KEY,
    post_id INT NOT NULL,
    user_id INT NOT NULL,
    parent_id INT NULL,
    content TEXT NOT NULL,
    is_approved BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES comments(id) ON DELETE CASCADE,
    INDEX idx_post_id (post_id),
    INDEX idx_user_id (user_id),
    INDEX idx_parent_id (parent_id),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB COMMENT='评论表';

-- 插入测试数据

-- 插入用户数据
INSERT INTO users (username, email, password_hash, first_name, last_name) VALUES
('admin', 'admin@example.com', '$2y$10$92IXUNpkjO0rOQ5byMi.Ye4oKoEa3Ro9llC/.og/at2.uheWG/igi', 'Admin', 'User'),
('john_doe', 'john@example.com', '$2y$10$92IXUNpkjO0rOQ5byMi.Ye4oKoEa3Ro9llC/.og/at2.uheWG/igi', 'John', 'Doe'),
('jane_smith', 'jane@example.com', '$2y$10$92IXUNpkjO0rOQ5byMi.Ye4oKoEa3Ro9llC/.og/at2.uheWG/igi', 'Jane', 'Smith'),
('bob_wilson', 'bob@example.com', '$2y$10$92IXUNpkjO0rOQ5byMi.Ye4oKoEa3Ro9llC/.og/at2.uheWG/igi', 'Bob', 'Wilson');

-- 插入标签数据
INSERT INTO tags (name, description, color) VALUES
('技术', '技术相关文章', '#007bff'),
('编程', '编程相关文章', '#28a745'),
('数据库', '数据库相关文章', '#ffc107'),
('Golang', 'Go语言相关文章', '#17a2b8'),
('MySQL', 'MySQL相关文章', '#6f42c1'),
('Docker', 'Docker相关文章', '#20c997'),
('教程', '教程类文章', '#fd7e14'),
('新闻', '新闻资讯', '#dc3545');

-- 插入文章数据
INSERT INTO posts (title, content, author_id, status, view_count, published_at) VALUES
('MySQL 命令行客户端开发指南', '本文介绍了如何使用 Golang 和 lipgloss 库开发一个现代化的 MySQL 命令行客户端工具...', 1, 'published', 156, '2024-01-15 10:30:00'),
('Golang 数据库操作最佳实践', '在 Golang 中操作数据库时需要注意的一些最佳实践和常见陷阱...', 2, 'published', 89, '2024-01-16 14:20:00'),
('Docker 容器化部署指南', '如何使用 Docker 容器化部署应用程序，包括 Dockerfile 编写和 docker-compose 配置...', 1, 'published', 234, '2024-01-17 09:15:00'),
('数据库设计原则', '良好的数据库设计是应用程序成功的基础，本文介绍了一些重要的设计原则...', 3, 'draft', 0, NULL),
('TUI 界面开发技巧', '终端用户界面开发的一些技巧和最佳实践...', 2, 'published', 67, '2024-01-18 16:45:00');

-- 插入文章标签关联
INSERT INTO post_tags (post_id, tag_id) VALUES
(1, 1), (1, 2), (1, 3), (1, 4), (1, 5),
(2, 2), (2, 4), (2, 3),
(3, 1), (3, 6), (3, 7),
(4, 3), (4, 7),
(5, 2), (5, 7);

-- 插入评论数据
INSERT INTO comments (post_id, user_id, content, is_approved) VALUES
(1, 2, '非常实用的工具，界面也很美观！', TRUE),
(1, 3, '请问支持其他数据库吗？', TRUE),
(1, 4, '代码结构很清晰，学习了！', TRUE),
(2, 1, 'Golang 的数据库操作确实需要特别注意这些点', TRUE),
(2, 3, '感谢分享，避免了很多坑', TRUE),
(3, 2, 'Docker 部署确实很方便', TRUE),
(5, 1, 'TUI 开发确实需要一些技巧', TRUE);

-- 显示创建的表
SHOW TABLES;
