echo on

pause

cd "C:\Rust\peer_to_peer"

pause

cargo build

pause

start target\debug\deps\peer_to_peer.exe "one"

pause

start target\debug\deps\peer_to_peer.exe "two"

pause

start target\debug\deps\peer_to_peer.exe "three"

pause

del target\debug\deps\peer_to_peer.exe

pause