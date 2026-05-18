# Broadcast

![alt text](image.png)
![alt text](image-1.png)

 Jalankan server terlebih dahulu dengan cargo run --bin server, kemudian buka terminal baru dan jalankan cargo run --bin client sebanyak 3 kali. Setiap pesan yang diketik di satu client akan di-broadcast ke semua client yang terhubung melalui server. Server menggunakan tokio::broadcast channel untuk mendistribusikan pesan ke semua subscriber. Seperti pada contoh, kita mengirim pesan di terminal paling bawah (lihat sebelah kanan), tapi message juga masuk di terminal ke-3.

 # changing the port
 Ada 2 file yang perlu dimodifikasi: server.rs (bagian TcpListener::bind) dan client.rs (bagian ClientBuilder::from_uri). Keduanya menggunakan protokol WebSocket (ws://), yang didefinisikan langsung di URI pada client dan implisit melalui ServerBuilder pada server. Protokol WebSocket berjalan di atas TCP, sehingga port diganti di kedua sisi koneksi.