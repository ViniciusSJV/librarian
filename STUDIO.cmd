@echo off
rem Librarian Studio: compila (release), sobe o servidor local e abre o navegador.
chcp 65001 >nul
cd /d "%~dp0"
cargo build --release --locked -p librarian-studio || (pause & exit /b 1)
start "" cmd /c "timeout /t 2 >nul & start http://127.0.0.1:8765/"
target\release\librarian_studio.exe %*
pause
