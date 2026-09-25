//! Один экземпляр на пользователя (спека этапа 7 §4.3). Первый экземпляр
//! слушает петлевой порт; второй подключается, просит показать окно и
//! закрывается. Порт на `127.0.0.1` — наружу не виден, брандмауэр не
//! спрашивает. Чужая программа на порту не ответит правильным приветствием —
//! тогда приложение запускается как обычно, без защиты.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

pub const PORT: u16 = 47631;
const HELLO: &str = "gacha-hub show";
const REPLY: &str = "gacha-hub ok";
const WAIT: Duration = Duration::from_millis(500);

pub enum Start {
    /// Порт свободен — этот экземпляр первый.
    First(TcpListener),
    /// Первый экземпляр уже работает и покажет своё окно — этому закрыться.
    Handed,
    /// Порт занят чужой программой — запускаться как обычно.
    Alone,
}

pub fn claim(port: u16) -> Start {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => Start::First(listener),
        Err(_) if ask_to_show(port) => Start::Handed,
        Err(_) => Start::Alone,
    }
}

/// Второй экземпляр: просит первый показать окно. `true` — первый ответил.
pub fn ask_to_show(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, WAIT) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(WAIT));
    if writeln!(stream, "{HELLO}").is_err() {
        return false;
    }
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).is_ok() && line.trim_end() == REPLY
}

/// Первый экземпляр: на каждое правильное приветствие отвечает и зовёт `on_show`.
pub fn serve(listener: TcpListener, on_show: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(WAIT));
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_ok() && line.trim_end() == HELLO {
                let _ = (&stream).write_all(format!("{REPLY}\n").as_bytes());
                on_show();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn free_port() -> u16 {
        TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port()
    }

    #[test]
    fn a_free_port_makes_this_instance_the_first() {
        assert!(matches!(claim(free_port()), Start::First(_)));
    }

    #[test]
    fn a_second_instance_hands_over_to_the_first() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        serve(listener, move || {
            let _ = tx.send(());
        });
        assert!(matches!(claim(port), Start::Handed));
        rx.recv_timeout(Duration::from_secs(2)).expect("первый экземпляр получил просьбу показать окно");
    }

    #[test]
    fn a_foreign_program_on_the_port_is_not_taken_for_us() {
        let foreign = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = foreign.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in foreign.incoming().flatten() {
                let _ = (&stream).write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
            }
        });
        assert!(matches!(claim(port), Start::Alone));
    }
}
