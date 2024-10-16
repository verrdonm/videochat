// Server message types

// enum PayloadType {
//     Offer,
//     Answer,
//     Candidate, 
//     Peers,
//     Echo,

// }

interface WsMessage {
    readonly recipient: string;
    readonly payload: OfferPayload | AnswerPayload | CandidatePayload | PeersPayload | EchoPayload | FilePayload;
}

interface OfferPayload {
    readonly sender: string;
    readonly offer: string;
}

interface AnswerPayload {
    readonly sender: string;
    readonly answer: string;
}

interface CandidatePayload {
    readonly sender: string;
    readonly candidate: string;
}

interface PeersPayload {
    readonly names: string[];
}

interface EchoPayload {
    readonly message: string;
}

interface FilePayload {
    readonly sender: string;
    readonly fileName: string;
    readonly fileSize: number;
}

// Local Data model
class IceStats {
    sent: number;
    received: number;
    constructor() {
      this.sent = 0;
      this.received = 0;
    }
  
    incSent(): void {
      this.sent++;
    }
  
    incReceived(): void {
      this.received++;
    }
  }
  
  class Peer {
    name: string;
    pc: RTCPeerConnection;
    ice: IceStats;
    dataChannel: RTCDataChannel;
    fileInfo: Object;
    constructor(name: string, pc: RTCPeerConnection, ice: IceStats, dataChannel: RTCDataChannel, fileInfo: Object) {
      this.name = name;
      this.pc = pc;
      this.ice = ice;
      this.dataChannel = dataChannel;
      this.fileInfo = fileInfo;
    }
  }
  
class ChatMessage {
    sender: string;
    msg: string;
    timestamp: number;
    constructor(sender: string, msg: string) {
      this.sender = sender;
      this.msg = msg;
      this.timestamp = Date.now();
    }
  }