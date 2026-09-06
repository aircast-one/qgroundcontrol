#version 440

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4  qt_Matrix;
    float qt_Opacity;
    vec4  tintColor;
    vec2  size;
    float radius;
    float minTint;
    float maxTint;
    float rimOpacity;
    float refraction;
    float blurRadius;
};

layout(binding = 1) uniform sampler2D source;

float sdRoundedBox(vec2 p, vec2 halfSize, float r)
{
    vec2 q = abs(p) - halfSize + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

// Glass is not a clear lens: what comes through is diffused. Thirteen taps on two rings is
// enough at this radius, costs one pass, and is what lets the tint come back down far enough
// for the content behind to stay perceptible.
vec4 blurredSample(vec2 uv, vec2 texel)
{
    const mat2 kRotate60 = mat2(0.5, 0.8660254, -0.8660254, 0.5);
    const mat2 kRotate30 = mat2(0.8660254, 0.5, -0.5, 0.8660254);

    vec2 ring1 = vec2(1.0, 0.0);
    vec2 ring2 = kRotate30 * ring1;
    vec4 sum = texture(source, uv) * 3.0;
    float weight = 3.0;
    for (int i = 0; i < 6; ++i) {
        sum += texture(source, clamp(uv + ring1 * texel * blurRadius * 0.55, vec2(0.0), vec2(1.0))) * 2.0;
        sum += texture(source, clamp(uv + ring2 * texel * blurRadius, vec2(0.0), vec2(1.0)));
        weight += 3.0;
        ring1 = kRotate60 * ring1;
        ring2 = kRotate60 * ring2;
    }
    return sum / weight;
}

void main()
{
    const vec3 kLuma = vec3(0.2126, 0.7152, 0.0722);

    vec2  p = (qt_TexCoord0 - 0.5) * size;
    float d = sdRoundedBox(p, size * 0.5, radius);

    vec2  grad  = vec2(dFdx(d), dFdy(d));
    float glen  = length(grad);
    vec2  normal = glen > 0.0001 ? grad / glen : vec2(0.0);
    float bend  = smoothstep(-max(radius, 1.0), 0.0, d);
    vec2  uv    = clamp(qt_TexCoord0 - normal * bend * refraction / size, vec2(0.0), vec2(1.0));

    vec4  src      = blurRadius > 0.0 ? blurredSample(uv, 1.0 / size) : texture(source, uv);
    float luma     = dot(src.rgb, kLuma);
    float tintLuma = dot(tintColor.rgb, kLuma);
    float amount   = mix(minTint, maxTint, smoothstep(0.15, 0.75, abs(luma - tintLuma)));
    vec3  glass    = mix(src.rgb, tintColor.rgb, amount);

    float shape = smoothstep(0.5, -0.5, d);
    float rim   = smoothstep(1.5, 0.0, abs(d + 0.75));

    glass += vec3(1.0) * rim * rimOpacity * (0.35 + 0.65 * (1.0 - qt_TexCoord0.y));

    fragColor = vec4(glass, 1.0) * shape * qt_Opacity;
}
