#include "http_parser.hpp"

#include <algorithm>
#include <cctype>
#include <charconv>
#include <limits>
#include <stdexcept>

namespace {

bool ascii_equal(std::string_view left, std::string_view right) noexcept {
    if (left.size() != right.size()) return false;
    for (std::size_t i = 0; i < left.size(); ++i) {
        auto a = static_cast<unsigned char>(left[i]);
        auto b = static_cast<unsigned char>(right[i]);
        if (a >= 'A' && a <= 'Z') a += 'a' - 'A';
        if (b >= 'A' && b <= 'Z') b += 'a' - 'A';
        if (a != b) return false;
    }
    return true;
}

bool token_char(unsigned char value) noexcept {
    return std::isalnum(value) || std::string_view("!#$%&'*+-.^_`|~").find(static_cast<char>(value)) != std::string_view::npos;
}

bool valid_value(std::string_view value) noexcept {
    return std::all_of(value.begin(), value.end(), [](unsigned char c) { return c == '\t' || (c >= 0x20 && c <= 0x7e); });
}

std::string_view trim_ows(std::string_view value) noexcept {
    while (!value.empty() && (value.front() == ' ' || value.front() == '\t')) value.remove_prefix(1);
    while (!value.empty() && (value.back() == ' ' || value.back() == '\t')) value.remove_suffix(1);
    return value;
}

} // namespace

std::optional<std::string_view> HttpRequest::header(std::string_view name) const noexcept {
    for (const auto& field : headers) if (ascii_equal(field.name, name)) return field.value;
    return std::nullopt;
}

RequestParser::RequestParser(std::size_t max_header_bytes, std::size_t max_body_bytes)
    : max_header_bytes_(max_header_bytes), max_body_bytes_(max_body_bytes) {
    if (max_header_bytes == 0 || max_body_bytes == 0) throw std::invalid_argument("parser limit must be positive");
}

bool RequestParser::push(std::string_view bytes) {
    if (error_ != ParserError::none) return false;
    pending_.append(bytes);

    const auto fail = [this](ParserError error) { error_ = error; pending_.clear(); return false; };
    for (;;) {
        const auto header_end = pending_.find("\r\n\r\n");
        if (header_end == std::string::npos) {
            for (std::size_t index = 0; index < pending_.size(); ++index) {
                if (pending_[index] == '\n' && (index == 0 || pending_[index - 1] != '\r')) return fail(ParserError::invalid_line_ending);
                if (pending_[index] == '\r' && index + 1 < pending_.size() && pending_[index + 1] != '\n') return fail(ParserError::invalid_line_ending);
            }
            if (pending_.size() > max_header_bytes_) return fail(ParserError::header_too_large);
            return true;
        }
        const std::size_t header_size = header_end + 4;
        if (header_size > max_header_bytes_) return fail(ParserError::header_too_large);

        const auto first_end = pending_.find("\r\n");
        if (first_end == std::string::npos) return fail(ParserError::invalid_request_line);
        const std::string_view request_line(pending_.data(), first_end);
        const auto first_space = request_line.find(' ');
        const auto second_space = first_space == std::string_view::npos ? std::string_view::npos : request_line.find(' ', first_space + 1);
        if (first_space == 0 || second_space == std::string_view::npos || request_line.find(' ', second_space + 1) != std::string_view::npos) return fail(ParserError::invalid_request_line);
        const auto method = request_line.substr(0, first_space);
        const auto target = request_line.substr(first_space + 1, second_space - first_space - 1);
        const auto version = request_line.substr(second_space + 1);
        if (!std::all_of(method.begin(), method.end(), [](unsigned char c) { return c >= 'A' && c <= 'Z'; }) ||
            target.empty() || target.front() != '/' || !std::all_of(target.begin(), target.end(), [](unsigned char c) { return c >= 0x21 && c <= 0x7e; }) || version != "HTTP/1.1") return fail(ParserError::invalid_request_line);

        HttpRequest request{std::string(method), std::string(target), {}, {}};
        std::optional<std::size_t> content_length;
        std::size_t line_start = first_end + 2;
        while (line_start < header_end) {
            const auto line_end = pending_.find("\r\n", line_start);
            if (line_end == std::string::npos || line_end > header_end) return fail(ParserError::invalid_line_ending);
            const std::string_view line(pending_.data() + line_start, line_end - line_start);
            const auto colon = line.find(':');
            if (colon == 0 || colon == std::string_view::npos || !std::all_of(line.begin(), line.begin() + static_cast<std::ptrdiff_t>(colon), token_char)) return fail(ParserError::invalid_header);
            const auto value = trim_ows(line.substr(colon + 1));
            if (!valid_value(value)) return fail(ParserError::invalid_header);
            const auto name = line.substr(0, colon);
            if (ascii_equal(name, "transfer-encoding")) return fail(ParserError::unsupported_transfer_encoding);
            if (ascii_equal(name, "content-length")) {
                if (value.empty() || !std::all_of(value.begin(), value.end(), [](unsigned char c) { return c >= '0' && c <= '9'; })) return fail(ParserError::invalid_content_length);
                std::size_t length{};
                const auto parsed = std::from_chars(value.data(), value.data() + value.size(), length);
                if (parsed.ec != std::errc{} || parsed.ptr != value.data() + value.size()) return fail(ParserError::invalid_content_length);
                if (length > max_body_bytes_) return fail(ParserError::body_too_large);
                if (content_length && *content_length != length) return fail(ParserError::conflicting_content_length);
                content_length = length;
            }
            request.headers.push_back({std::string(name), std::string(value)});
            line_start = line_end + 2;
        }
        const auto body_size = content_length.value_or(0);
        if (pending_.size() < header_size + body_size) return true;
        request.body.assign(pending_.data() + header_size, body_size);
        completed_.push_back(std::move(request));
        pending_.erase(0, header_size + body_size);
    }
}

std::optional<HttpRequest> RequestParser::take_request() {
    if (completed_.empty()) return std::nullopt;
    HttpRequest result = std::move(completed_.front());
    completed_.erase(completed_.begin());
    return result;
}

ParserError RequestParser::error() const noexcept { return error_; }
bool RequestParser::empty() const noexcept { return completed_.empty(); }
