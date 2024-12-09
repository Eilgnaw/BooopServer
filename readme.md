APP运行后生成 UUID 或从缓存获取
连接 TCP 上报 ID
TCPserver 存储 ID


HTTP 发送请求后 根据携带的 UUID 做个tcp 客户端向指定客户端 发送消息 
客户端收到消息后播放动画


curl -X POST http://127.0.0.1:3030/send_message \
     -H "Content-Type: application/json" \
     -d '{"cmd": "send","to": "8955D561-895C-44A2-8C99-7459CE826392", "emoji": "shen"}'

tokio = { version = "1.0", features = ["full"] }
warp = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
