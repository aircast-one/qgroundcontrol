require "json"
require "jwt"
require "net/http"
require "openssl"

API = "https://api.appstoreconnect.apple.com".freeze
CHECKS = [
  "/v1/apps?filter[bundleId]=one.aircast.app&fields[apps]=bundleId",
  "/v1/bundleIds?filter[identifier]=one.aircast.app&fields[bundleIds]=identifier",
].freeze
TO_ACCEPT = "the account holder must accept the Apple Developer Program License Agreement at " \
            "https://developer.apple.com/account and any pending agreement at " \
            "https://appstoreconnect.apple.com/business".freeze

def bearer(key_id, issuer, key_path)
  now = Time.now.to_i
  claims = { iss: issuer, iat: now, exp: now + 600, aud: "appstoreconnect-v1" }
  JWT.encode(claims, OpenSSL::PKey::EC.new(File.read(key_path)), "ES256", { kid: key_id, typ: "JWT" })
end

def answer(path, token)
  response = Net::HTTP.get_response(URI(API + path), { "Authorization" => "Bearer #{token}" })
  return [] if response.is_a?(Net::HTTPSuccess)

  JSON.parse(response.body).fetch("errors", [{ "code" => response.code, "detail" => response.message }])
rescue JSON::ParserError, SystemCallError, Net::OpenTimeout, Net::ReadTimeout => e
  [{ "code" => "UNREACHABLE", "detail" => e.message }]
end

def unaccepted(errors)
  errors.select { |error| error["code"].to_s.include?("AGREEMENT") }
end

def verdict(errors, rerun)
  missing = unaccepted(errors)
  return [1, ["::error title=Apple agreement not accepted::#{missing.first['detail']} Before iOS can ship, #{TO_ACCEPT}, then rerun: #{rerun}"]] unless missing.empty?

  [0, errors.map { |error| "::warning title=App Store Connect check::#{error['code']} #{error['detail']}" }]
end

if __FILE__ == $PROGRAM_NAME
  token = bearer(ENV.fetch("ASC_KEY_ID"), ENV.fetch("ASC_ISSUER_ID"), ENV.fetch("ASC_KEY_PATH"))
  status, lines = verdict(CHECKS.flat_map { |path| answer(path, token) },
                          "gh run rerun #{ENV.fetch('GITHUB_RUN_ID', '<run id>')} --failed")
  puts lines
  exit status
end
