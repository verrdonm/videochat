// Server message types
// Local Data model
var IceStats = /** @class */ (function () {
    function IceStats() {
        this.sent = 0;
        this.received = 0;
    }
    IceStats.prototype.incSent = function () {
        this.sent++;
    };
    IceStats.prototype.incReceived = function () {
        this.received++;
    };
    return IceStats;
}());
var Peer = /** @class */ (function () {
    function Peer(name, pc, ice, dataChannel, fileInfo) {
        this.name = name;
        this.pc = pc;
        this.ice = ice;
        this.dataChannel = dataChannel;
        this.fileInfo = fileInfo;
    }
    return Peer;
}());
var ChatMessage = /** @class */ (function () {
    function ChatMessage(sender, msg) {
        this.sender = sender;
        this.msg = msg;
        this.timestamp = Date.now();
    }
    return ChatMessage;
}());
