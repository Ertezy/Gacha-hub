//! Один экземпляр на пользователя (спека этапа 7 §4.3). Первый экземпляр
//! слушает петлевой порт; второй подключается, называет себя и просит
//! показать окно. Порт на `127.0.0.1` — наружу не виден, брандмауэр не
//! спрашивает.
//!
//! Порт общий на весь компьютер, а не на пользователя — с быстрым
//! переключением пользователей Windows экземпляр другого пользователя мог бы
//! ответить вместо своего и показать чужое окно в чужой сессии. Поэтому
//! приветствие несёт личность (`USERDOMAIN\USERNAME`), и первый экземпляр
//! отвечает, только если она совпадает с его собственной; иначе — как и с
//! чужой программой на порту, которая ответит неправильным приветствием или
//! не ответит вовсе, — соединение закрывается без ответа, и второй экземпляр
//! запускается как обычно, без защиты.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

/// Порт сменился вместе с переименованием: прежняя версия (Gacha Hub) слушала
/// 47631, и общий порт лишил бы Kitsudock защиты от второго экземпляра, пока
/// запущен Gacha Hub. Теперь у двух приложений порты никогда не совпадают.
pub const PORT: u16 = 47632;
const HELLO: &str = "kitsudock show";
const REPLY: &str = "kitsudock ok";
const WAIT: Duration = Duration::from_millis(500);

pub enum Start {
    /// Порт свободен — этот экземпляр первый.
    First(TcpListener),
    /// Первый экземпляр уже работает и покажет своё окно — этому закрыться.
    Handed,
    /// Порт занят чужой программой или другим пользователем — запускаться
    /// как обычно.
    Alone,
}

/// Личность пользователя для приветствия: `USERDOMAIN\USERNAME` из
/// переменных окружения. Нет переменной — пустая строка на её месте вместо
/// паники: единая личность на весь компьютер хуже, чем задуманная защита на
/// пользователя, но лучше, чем совсем без единого экземпляра.
pub fn current_identity() -> String {
    format!(
        "{}\\{}",
        std::env::var("USERDOMAIN").unwrap_or_default(),
        std::env::var("USERNAME").unwrap_or_default(),
    )
}

fn greeting(identity: &str) -> String {
    format!("{HELLO} {identity}")
}

pub fn claim(port: u16, identity: &str) -> Start {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => Start::First(listener),
        Err(_) if ask_to_show(port, identity) => Start::Handed,
        Err(_) => Start::Alone,
    }
}

/// Второй экземпляр: просит первый показать окно. `true` — первый ответил.
pub fn ask_to_show(port: u16, identity: &str) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, WAIT) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(WAIT));
    if writeln!(stream, "{}", greeting(identity)).is_err() {
        return false;
    }
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).is_ok() && line.trim_end() == REPLY
}

/// Первый экземпляр: на приветствие со своей личностью отвечает и зовёт
/// `on_show`; на приветствие с чужой (другой пользователь на том же порту) —
/// не отвечает, соединение просто закрывается.
pub fn serve(listener: TcpListener, identity: String, on_show: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let expected = greeting(&identity);
        for stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(WAIT));
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_ok() && line.trim_end() == expected
            {
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

    const WHO: &str = r"TESTDOMAIN\tester";

    fn free_port() -> u16 {
        TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port()
    }

    #[test]
    fn the_port_is_not_the_one_gacha_hub_listened_on() {
        // Прежняя версия слушала 47631: общий порт лишил бы Kitsudock защиты
        // от второго экземпляра, пока Gacha Hub запущен.
        assert_ne!(PORT, 47631);
    }

    #[test]
    fn a_free_port_makes_this_instance_the_first() {
        assert!(matches!(claim(free_port(), WHO), Start::First(_)));
    }

    #[test]
    fn a_second_instance_hands_over_to_the_first() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        serve(listener, WHO.to_string(), move || {
            let _ = tx.send(());
        });
        assert!(matches!(claim(port, WHO), Start::Handed));
        rx.recv_timeout(Duration::from_secs(2)).expect("первый экземпляр получил просьбу показать окно");
    }

    #[test]
    fn a_different_users_greeting_gets_no_reply() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel::<()>();
        serve(listener, WHO.to_string(), move || {
            let _ = tx.send(());
        });
        assert!(matches!(claim(port, r"OTHERDOMAIN\other"), Start::Alone));
        assert!(rx.try_recv().is_err(), "окно чужого пользователя показывать не просили");
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
        assert!(matches!(claim(port, WHO), Start::Alone));
    }
}
