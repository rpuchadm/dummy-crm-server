pub async fn initialization(pool: sqlx::Pool<sqlx::Postgres>) {
    sqlx::query(
        r#"        
        DROP TABLE IF EXISTS issue_request_articulos;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        DROP TABLE IF EXISTS issue_request_clientes;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        DROP TABLE IF EXISTS issue_request_pedidos;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        DROP TABLE IF EXISTS issue_request;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        DROP TABLE IF EXISTS pedidos_detalles;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        DROP TABLE IF EXISTS pedidos;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        DROP TABLE IF EXISTS clientes;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        DROP TABLE IF EXISTS articulos;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        CREATE TABLE IF NOT EXISTS clientes (
            id SERIAL PRIMARY KEY,              -- Identificador único del cliente
            user_id INT NOT NULL UNIQUE,        -- Identificador único del usuario, integra con auth
            nombre VARCHAR(100) NOT NULL,       -- Nombre del cliente
            email VARCHAR(100) UNIQUE,          -- Email del cliente (único)
            telefono VARCHAR(20),               -- Teléfono del cliente
            direccion TEXT,                     -- Dirección del cliente
            fecha_registro TIMESTAMP DEFAULT CURRENT_TIMESTAMP -- Fecha de registro
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        CREATE TABLE IF NOT EXISTS articulos (
            id SERIAL PRIMARY KEY,              -- Identificador único del artículo
            nombre VARCHAR(100) NOT NULL,       -- Nombre del artículo
            descripcion TEXT,                   -- Descripción del artículo
            precio INT NOT NULL,     -- Precio del artículo
            stock INT NOT NULL DEFAULT 0,       -- Cantidad en stock
            fecha_creacion TIMESTAMP DEFAULT CURRENT_TIMESTAMP -- Fecha de creación
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        CREATE TABLE IF NOT EXISTS pedidos (
            id SERIAL PRIMARY KEY,              -- Identificador único del pedido
            cliente_id INT NOT NULL,             -- ID del cliente que realiza el pedido
            fecha_pedido TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Fecha del pedido
            estado VARCHAR(50) NOT NULL DEFAULT 'Pendiente', -- Estado del pedido (Pendiente, Enviado, Entregado, etc.)
            total INT NOT NULL,      -- Total del pedido
            FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        CREATE TABLE IF NOT EXISTS pedidos_detalles (
            id SERIAL PRIMARY KEY,              -- Identificador único del detalle
            pedido_id INT NOT NULL,             -- ID del pedido
            articulo_id INT NOT NULL,           -- ID del artículo
            cantidad INT NOT NULL,              -- Cantidad del artículo en el pedido
            precio_unitario INT NOT NULL, -- Precio unitario del artículo en el momento del pedido
            subtotal INT NOT NULL,   -- Subtotal (cantidad * precio_unitario)
            FOREIGN KEY (pedido_id) REFERENCES pedidos(id) ON DELETE CASCADE,
            FOREIGN KEY (articulo_id) REFERENCES articulos(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        CREATE TABLE IF NOT EXISTS issue_request (
            id SERIAL PRIMARY KEY,              -- Identificador único
            fecha_creacion TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Fecha de creación
            data JSONB NOT NULL,             -- Datos de la solicitud
            issue_id INT                  -- ID del issue (opcional)
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        CREATE TABLE IF NOT EXISTS issue_request_articulos (
            issue_request_id INT NOT NULL,             -- ID de la solicitud
            articulo_id INT NOT NULL,                  -- ID del artículo
            fecha_creacion TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Fecha de creación
            FOREIGN KEY (issue_request_id) REFERENCES issue_request(id) ON DELETE CASCADE,
            FOREIGN KEY (articulo_id) REFERENCES articulos(id) ON DELETE CASCADE,
            PRIMARY KEY (issue_request_id, articulo_id) -- Clave primaria compuesta
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        CREATE TABLE IF NOT EXISTS issue_request_clientes (
            issue_request_id INT NOT NULL,             -- ID de la solicitud
            cliente_id INT NOT NULL,                  -- ID del cliente
            fecha_creacion TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Fecha de creación
            FOREIGN KEY (issue_request_id) REFERENCES issue_request(id) ON DELETE CASCADE,
            FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON DELETE CASCADE,
            PRIMARY KEY (issue_request_id, cliente_id) -- Clave primaria compuesta
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        CREATE TABLE IF NOT EXISTS issue_request_pedidos (
            issue_request_id INT NOT NULL,             -- ID de la solicitud
            pedido_id INT NOT NULL,                  -- ID del pedido
            fecha_creacion TIMESTAMP DEFAULT CURRENT_TIMESTAMP, -- Fecha de creación
            FOREIGN KEY (issue_request_id) REFERENCES issue_request(id) ON DELETE CASCADE,
            FOREIGN KEY (pedido_id) REFERENCES pedidos(id) ON DELETE CASCADE,
            PRIMARY KEY (issue_request_id, pedido_id) -- Clave primaria compuesta
        );
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        INSERT INTO clientes (user_id,nombre, email, telefono, direccion)
        VALUES (1,'Juan Pérez', 'juan@example.com', '123456789', 'Calle Falsa 123');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        INSERT INTO articulos (nombre, descripcion, precio, stock)
        VALUES ('Laptop', 'Laptop de 15 pulgadas', 120000, 10);
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        INSERT INTO pedidos (cliente_id, total)
        VALUES (1, 240000); -- Suponiendo que el cliente con ID 1 compra 2 laptops
    "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"        
        INSERT INTO pedidos_detalles (pedido_id, articulo_id, cantidad, precio_unitario, subtotal)
        VALUES (1, 1, 2, 120000, 240000); -- 2 laptops a 1200.00 cada una
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
}
