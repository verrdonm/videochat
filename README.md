# Video Chat

This basic single-executable WebRTC video chat app is essentially the combination of a variety of tutorials and examples, to learn about a variety of techs and patterns. This is a works-on-my-computer level of implementation. It also works deployed to a server, but there are a bunch of tablestakes production application features that aren't present.

## Currently implemented
- Axum websocket server
-- Axum middleware to supply a service the holds state
-- rustls to serve https
- Askama html page serving the landing page for the WebRTC app
- Multi-stage dockerfile to build for the linux architecture, and Distroless runtime container

## Future projects
- A less naive signalling server state machine
- A host of UI/UX considerations
-- Try out htmx to serve what amounts to components
-- Track interactions to mute outgoing or incoming webrtc media by user
- Server WebRTC participation. webrtc-rs has capabilities here, but the server here is just facilitating p2p interactions
- Auth