// import navigator from 'webrtc-adapter';
import {ChatMessage, IceStats, Peer} from './types.js'; // TODO fix this for both ts and js

export default class WebRTCSession {
    startButton: HTMLButtonElement;
    hangupButton: HTMLButtonElement;
    muteButton: HTMLButtonElement;
    chatButton: HTMLButtonElement;
    chatBox: HTMLDivElement;

    nameInput: HTMLInputElement;
    roomInput: HTMLInputElement;

    remoteVideos: HTMLDivElement;
    localVideo: HTMLVideoElement;

    myName: string;

    peers: Map<string, Peer>;
    ws: WebSocket;
    localStream: MediaStream;

    constructor() {
        this.startButton = document.getElementById('startButton')! as HTMLButtonElement;
        this.hangupButton = document.getElementById('hangupButton')! as HTMLButtonElement;
        this.muteButton = document.getElementById('muteButton')! as HTMLButtonElement;
        this.chatButton = document.getElementById('chatButton')! as HTMLButtonElement;
        this.chatBox = document.getElementById('chat')! as HTMLDivElement;

        this.remoteVideos = document.getElementById('remotePeers')! as HTMLDivElement;
        this.localVideo = document.getElementById('localVideo')! as HTMLVideoElement;

        this.nameInput = document.getElementById('name')! as HTMLInputElement;
        this.roomInput = document.getElementById('room')! as HTMLInputElement;

        this.hangupButton.disabled = true;
        this.startButton.onclick = this.start;
        this.hangupButton.onclick = this.hangup;
        this.muteButton.onclick = this.mute;

        this.peers = new Map<string, Peer>();
    }

    signal = (recipient: string, msg: Object) => {
        console.log(`sending message to ${recipient}: ${JSON.stringify(msg)}`);
        this.ws.send(JSON.stringify({"recipient": recipient, "payload": msg}));
    }

    waitForOpen = async () => {
        while(!(this.ws && this.ws.readyState == WebSocket.OPEN)) {
          console.log(`waiting for WebSocket.OPEN`);
          await new Promise(r => setTimeout(r, 200));
        }
      }

    start = async () => {
        this.localStream = await navigator.mediaDevices.getUserMedia({audio: true, video: true});
        this.localVideo.srcObject = this.localStream;
        
        this.startButton.disabled = true;
        this.hangupButton.disabled = false;
        
        this.myName = this.nameInput.value;
        let roomCode = this.roomInput.value;
        this.nameInput.disabled = true;
        this.roomInput.disabled = true;
        
        this.ws = new WebSocket(`/ws/${roomCode}/${this.myName}`);
        await this.waitForOpen();
        this.signal(this.myName, {"echo": {"message": "hello"}}); // TODO message type
        this.ws.onmessage = async (event) => {
            let text = event;
            try {
            console.log(`Received: ${event.data}`);
            const msg = JSON.parse(event.data);
            let payload = msg.payload;
            if (payload.peers) {
                payload.peers.names.forEach(async (peerName) => {
                if (this.peers[peerName]) return;
                var pc = await this.createPeerConnection(peerName, true);
                var offer = await pc.createOffer();
                await pc.setLocalDescription(offer);
                this.signal(peerName, {"offer":{"sender":this.myName, "payload": btoa(JSON.stringify(offer))}});
                });
            } else if (payload.offer) {
                if (this.peers[payload.offer.sender]) {return;}
        
                var pc = await this.createPeerConnection(payload.offer.sender, false);
                await pc.setRemoteDescription(JSON.parse(atob(payload.offer.payload)));
                let answer = await pc.createAnswer();
                this.signal(payload.offer.sender, {"answer":{"sender":this.myName, "payload": btoa(JSON.stringify(answer))}});
                await pc.setLocalDescription(answer);
            } else if(payload.answer) {
                var peerName = payload.answer.sender;
                await this.peers[peerName].pc.setRemoteDescription(JSON.parse(atob(payload.answer.payload)));
            } else if(payload.candidate) {
                var peer = this.peers[payload.candidate.sender];
                var pc = peer.pc;
                if (!pc) {
                console.log(`Received candidate from peer ${payload.candidate.sender} with no connection`);
                return;
                }
                while(!pc.remoteDescription) {
                console.log(`Waiting for remote description from ${payload.candidate.sender}`);
                await new Promise(r => setTimeout(r, 100));
                }
                await pc.addIceCandidate(JSON.parse(atob(payload.candidate.payload)));
                peer.ice.incReceived();
            } else if(payload.file) {
                // TODO reenable file sending
                // await fileInbound(payload.file);
            }
            } catch (e) {
                console.log("failed to parse event data: ", e);
            }
        };
    }

    createPeerConnection = async (peerName, dc) => {
        console.log(`creating peer connection with ${peerName}`);
        var pc = new RTCPeerConnection({
        iceServers: [
            {
            urls: 'turn:44.243.60.163:3478',
            username: "barfolomew",
            credential: "thatsgoingtoleaveamark"
            },
            {
            urls: 'stun:stun.l.google.com:19302'
            }
        ]});
        
        var peer = new Peer(peerName, pc, new IceStats(), null, null);
    
        // TODO Refactor data channel so that chat and files are separate
        // if (dc) {
        //     peer.dataChannel = await createDataChannel(peerName, peer);
        // } else {
        //     pc.ondatachannel = async (ev) => {
        //         console.log(`Received data channel from ${peerName}`);
        //         var dataChannel = ev.channel;
        //         peer.dataChannel = dataChannel;
        //         // dataChannel.onmessage = await onReceiveMessageFromPeer(peerName);
        //         dataChannel.onmessage = onReceiveChatMessage;
        //         dataChannel.onopen = await onDataChannelStateChange(peerName);
        //         dataChannel.onclose = await onDataChannelStateChange(peerName);
        //         receivedSize = 0;
        //         bitrateMax = 0;
        //         downloadAnchor.textContent = '';
        //         downloadAnchor.removeAttribute('download');
        //         if (downloadAnchor.href) {
        //         URL.revokeObjectURL(downloadAnchor.href);
        //         downloadAnchor.removeAttribute('href');
        //         }
        //     };
        // }
    
        pc.onicecandidate = async e => {
        if (e.candidate) {
            await this.waitForOpen();
            console.log(JSON.stringify(pc));
            while(!pc.remoteDescription) {
            console.log(`waiting for remote description from ${peerName} in onicecandidate: ${pc.remoteDescription}`);
            await new Promise(r => setTimeout(r, 400));
            }
            this.signal(peer.name, {"candidate": {"sender": this.myName, "payload": btoa(JSON.stringify(e.candidate))}});
            peer.ice.incSent();
        }
        };
    
    
        var videoElementId = 'remoteVideo' + peerName;
        var videoElement = document.getElementById(videoElementId)! as HTMLVideoElement;
    
        if (!videoElement) {
            videoElement = document.createElement('video') as HTMLVideoElement;
            videoElement.id = 'remoteVideo' + peerName;
            videoElement.playsInline = true;
            videoElement.autoplay = true;
            this.remoteVideos.appendChild(videoElement);
        }
    
        pc.ontrack = e => {
            console.log(`on track: ${JSON.stringify(e)}`);
            videoElement.srcObject = e.streams[0];
        };
    
        this.localStream.getTracks().forEach(track => pc.addTrack(track, this.localStream));
        this.peers[peerName] = peer;
        return pc;
    }

    hangup = async () => {
        for (let [n, peer] of this.peers) {
          console.log(`hanging up ${n} connection ${peer}`);
          if (peer && peer.pc) {
            var pc = peer.pc;
            pc.getSenders().forEach((sender) => {
              sender.track.stop();
            });
            pc.close();
            pc = null;
          }
          let v = document.getElementById('remoteVideo' + n);
          if (v) {
            v.remove();
          }
          // remoteVideos.removeChild(removeVideo);
        }
        this.peers.clear();
        this.remoteVideos.innerHTML="";
        this.startButton.disabled = false;
        this.hangupButton.disabled = true;
        this.nameInput.disabled = false;
        this.roomInput.disabled = false;
        this.localStream.getTracks().forEach(track => track.stop());
        this.localStream = null;
        if (this.ws) {
          this.ws.close();
        }
    };

    mute = () => {
        this.muteButton.innerText = this.localStream.getAudioTracks()[0].enabled ? "Unmute" : "Mute";
        this.localStream.getAudioTracks()[0].enabled=!this.localStream.getAudioTracks()[0].enabled;
    }
}
