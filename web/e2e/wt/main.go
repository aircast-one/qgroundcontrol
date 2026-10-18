package main

import (
	"crypto/tls"
	"encoding/binary"
	"encoding/json"
	"log"
	"math"
	"net/http"
	"strings"
	"sync/atomic"
	"time"

	"github.com/quic-go/quic-go"
	"github.com/quic-go/quic-go/http3"
	"github.com/quic-go/webtransport-go"
)

const (
	addr          = "localhost:18443"
	listen        = ":18443"
	accessToken   = "e2e-access"
	relayPrefix   = "e2e-relay-"
	tick          = 250 * time.Millisecond
	vehicleSystem = 1
	homeLat       = 47.3977
	homeLon       = 8.5456
)

var gcsHeartbeats atomic.Int64

func crcX25(bytes []byte) uint16 {
	crc := uint16(0xffff)
	for _, b := range bytes {
		mixed := b ^ byte(crc)
		mixed ^= mixed << 4
		crc = (crc >> 8) ^ (uint16(mixed) << 8) ^ (uint16(mixed) << 3) ^ (uint16(mixed) >> 4)
	}
	return crc
}

func frame(seq byte, id uint32, crcExtra byte, payload []byte) []byte {
	header := []byte{byte(len(payload)), 0, 0, seq, vehicleSystem, 1, byte(id), byte(id >> 8), byte(id >> 16)}
	body := append(append(append([]byte{}, header...), payload...), crcExtra)
	crc := crcX25(body)
	out := append([]byte{0xfd}, header...)
	out = append(out, payload...)
	return append(out, byte(crc), byte(crc>>8))
}

func heartbeat(seq byte) []byte {
	return frame(seq, 0, 50, []byte{0, 0, 0, 0, 2, 12, 1, 4, 3})
}

func position(seq byte, n int) []byte {
	payload := make([]byte, 28)
	binary.LittleEndian.PutUint32(payload[0:], uint32(n*250))
	binary.LittleEndian.PutUint32(payload[4:], uint32(int32(math.Round((homeLat+float64(n)*0.00001)*1e7))))
	binary.LittleEndian.PutUint32(payload[8:], uint32(int32(math.Round(homeLon*1e7))))
	binary.LittleEndian.PutUint32(payload[12:], uint32(int32(500_000)))
	binary.LittleEndian.PutUint32(payload[16:], uint32(int32(20_000)))
	return frame(seq, 33, 104, payload)
}

func countGcsHeartbeats(bytes []byte) int64 {
	if len(bytes) < 12 || bytes[0] != 0xfd {
		return 0
	}
	id := uint32(bytes[7]) | uint32(bytes[8])<<8 | uint32(bytes[9])<<16
	if id == 0 && bytes[5] != vehicleSystem {
		return 1
	}
	return 0
}

func cors(w http.ResponseWriter, r *http.Request) bool {
	w.Header().Set("Access-Control-Allow-Origin", r.Header.Get("Origin"))
	w.Header().Set("Access-Control-Allow-Headers", "Authorization, Content-Type")
	w.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
	if r.Method == http.MethodOptions {
		w.WriteHeader(http.StatusNoContent)
		return true
	}
	return false
}

func reply(w http.ResponseWriter, body any) {
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(body)
}

func rest(mux *http.ServeMux) {
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if cors(w, r) {
			return
		}
		switch {
		case r.URL.Path == "/stats":
			reply(w, map[string]int64{"gcsHeartbeats": gcsHeartbeats.Load()})
		case r.URL.Path == "/v1/oauth2/cli/code":
			reply(w, map[string]any{"device_code": "e2e-device", "user_code": "E2E-0001", "verification_uri_complete": "https://" + addr + "/approve", "interval": 1})
		case r.URL.Path == "/v1/oauth2/cli/token":
			reply(w, map[string]string{"access_token": accessToken})
		case strings.HasPrefix(r.URL.Path, "/v1/user/devices/") && strings.HasSuffix(r.URL.Path, "/webtransport-token"):
			if r.Header.Get("Authorization") != "Bearer "+accessToken {
				w.WriteHeader(http.StatusUnauthorized)
				return
			}
			device := strings.TrimSuffix(strings.TrimPrefix(r.URL.Path, "/v1/user/devices/"), "/webtransport-token")
			reply(w, map[string]string{"token": relayPrefix + device, "expires_at": time.Now().Add(5 * time.Minute).UTC().Format(time.RFC3339)})
		default:
			reply(w, map[string]bool{"ok": true})
		}
	})
}

func relay(server *webtransport.Server) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		device, browser := strings.CutSuffix(strings.TrimPrefix(r.URL.Path, "/v1/mavlink/device/"), "/webtransport")
		if !browser {
			w.WriteHeader(http.StatusNotFound)
			return
		}
		if r.URL.Query().Get("token") != relayPrefix+device {
			w.WriteHeader(http.StatusUnauthorized)
			return
		}
		session, err := server.Upgrade(w, r)
		if err != nil {
			log.Printf("upgrade: %v", err)
			return
		}
		ctx := session.Context()
		go func() {
			for {
				bytes, err := session.ReceiveDatagram(ctx)
				if err != nil {
					return
				}
				gcsHeartbeats.Add(countGcsHeartbeats(bytes))
			}
		}()
		ticker := time.NewTicker(tick)
		defer ticker.Stop()
		for n := 0; ; n++ {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				if n%4 == 0 {
					_ = session.SendDatagram(heartbeat(byte(n * 2)))
				}
				_ = session.SendDatagram(position(byte(n*2+1), n))
			}
		}
	}
}

func main() {
	cert, err := tls.LoadX509KeyPair("cert.pem", "key.pem")
	if err != nil {
		log.Fatal(err)
	}
	tlsConfig := &tls.Config{Certificates: []tls.Certificate{cert}}
	mux := http.NewServeMux()
	rest(mux)
	go func() {
		log.Fatal((&http.Server{Addr: listen, Handler: mux, TLSConfig: tlsConfig}).ListenAndServeTLS("", ""))
	}()
	h3 := &http3.Server{Addr: listen, TLSConfig: http3.ConfigureTLSConfig(tlsConfig), QUICConfig: &quic.Config{EnableDatagrams: true, KeepAlivePeriod: 10 * time.Second}, Handler: mux, EnableDatagrams: true}
	webtransport.ConfigureHTTP3Server(h3)
	server := &webtransport.Server{H3: h3, CheckOrigin: func(*http.Request) bool { return true }}
	mux.HandleFunc("/v1/mavlink/device/", relay(server))
	log.Fatal(server.ListenAndServe())
}
