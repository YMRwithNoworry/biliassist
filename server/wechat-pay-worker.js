/**
 * BilibiliAccountManager - 微信支付中间层
 * 
 * 部署到 Cloudflare Workers（免费计划即可）
 * 
 * 环境变量 (在 Cloudflare Dashboard 中设置):
 *   WECHAT_APPID      - 微信支付 AppID
 *   WECHAT_MCHID      - 微信支付商户号
 *   WECHAT_API_KEY    - 微信支付 API v2 密钥 (32位)
 *   SUPABASE_URL      - Supabase 项目 URL
 *   SUPABASE_SERVICE_KEY - Supabase service_role key（用于更新 payments 表）
 * 
 * 部署命令:
 *   npm install -g wrangler
 *   wrangler deploy server/wechat-pay-worker.js --name bam-pay
 */

// ============================================================
//  工具函数
// ============================================================

function xmlToJson(xmlText) {
  const obj = {}
  const matches = xmlText.matchAll(/<(\w+)>([^<]+)<\/\1>/g)
  for (const m of matches) obj[m[1]] = m[2]
  return obj
}

function jsonToXml(obj) {
  let xml = '<xml>'
  for (const [k, v] of Object.entries(obj)) xml += `<${k}>${v}</${k}>`
  return xml + '</xml>'
}

function md5Sign(params, apiKey) {
  const sorted = Object.keys(params).sort()
  let str = sorted.map(k => `${k}=${params[k]}`).join('&') + `&key=${apiKey}`
  return hexMD5(str).toUpperCase()
}

function hexMD5(str) {
  const data = new TextEncoder().encode(str)
  let hash = ''
  // Use Web Crypto API
  return crypto.subtle.digest('MD5', data).then(buf => {
    const bytes = new Uint8Array(buf)
    return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('')
  })
}

async function md5SignAsync(params, apiKey) {
  const sorted = Object.keys(params).sort()
  let str = sorted.map(k => `${k}=${params[k]}`).join('&') + `&key=${apiKey}`
  const hash = await hexMD5(str)
  return hash.toUpperCase()
}

function nonceStr() {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
  let result = ''
  for (let i = 0; i < 32; i++) result += chars[Math.floor(Math.random() * chars.length)]
  return result
}

function nowStr() {
  const d = new Date()
  return d.getFullYear().toString() +
    String(d.getMonth() + 1).padStart(2, '0') +
    String(d.getDate()).padStart(2, '0') +
    String(d.getHours()).padStart(2, '0') +
    String(d.getMinutes()).padStart(2, '0') +
    String(d.getSeconds()).padStart(2, '0')
}

// ============================================================
//  Supabase 操作
// ============================================================

async function upsertPayment(supabaseUrl, serviceKey, userId, outTradeNo, status) {
  const resp = await fetch(`${supabaseUrl}/rest/v1/payments`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'apikey': serviceKey,
      'Authorization': `Bearer ${serviceKey}`,
      'Prefer': 'resolution=merge-duplicates'
    },
    body: JSON.stringify({
      user_id: userId,
      out_trade_no: outTradeNo,
      status: status
    })
  })
  if (!resp.ok) {
    const text = await resp.text()
    throw new Error(`Supabase 更新失败: ${resp.status} ${text}`)
  }
  return true
}

async function getPaymentByOrder(supabaseUrl, serviceKey, outTradeNo) {
  const resp = await fetch(
    `${supabaseUrl}/rest/v1/payments?out_trade_no=eq.${encodeURIComponent(outTradeNo)}&select=*`,
    {
      headers: {
        'apikey': serviceKey,
        'Authorization': `Bearer ${serviceKey}`
      }
    }
  )
  if (!resp.ok) return null
  const data = await resp.json()
  return data?.[0] || null
}

async function upsertUserTier(supabaseUrl, serviceKey, userId, tier) {
  const resp = await fetch(`${supabaseUrl}/rest/v1/user_tiers`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'apikey': serviceKey,
      'Authorization': `Bearer ${serviceKey}`,
      'Prefer': 'resolution=merge-duplicates'
    },
    body: JSON.stringify({
      user_id: userId,
      tier: tier
    })
  })
  if (!resp.ok) {
    const text = await resp.text()
    throw new Error(`更新用户等级失败: ${resp.status} ${text}`)
  }
  return true
}

// ============================================================
//  路由处理
// ============================================================

async function handleCreateOrder(request, env) {
  const { user_id } = await request.json()
  if (!user_id) {
    return new Response(JSON.stringify({ error: '缺少 user_id' }), {
      status: 400, headers: { 'Content-Type': 'application/json' }
    })
  }

  const outTradeNo = `BAM${nowStr()}${String(Math.random()).slice(2, 8)}`
  const spbillCreateIp = request.headers.get('CF-Connecting-IP') || '127.0.0.1'
  const notifyUrl = `${new URL(request.url).origin}/wechat-notify`

  const params = {
    appid: env.WECHAT_APPID,
    mch_id: env.WECHAT_MCHID,
    nonce_str: nonceStr(),
    body: 'BilibiliAccountManager Plus 升级',
    out_trade_no: outTradeNo,
    total_fee: '1', // 1 分 = 0.01 元（测试用，正式改为 600 = 6 元）
    spbill_create_ip: spbillCreateIp,
    notify_url: notifyUrl,
    trade_type: 'NATIVE',
    product_id: 'BAM_REGISTER_001'
  }

  const sign = await md5SignAsync(params, env.WECHAT_API_KEY)
  params.sign = sign

  const xmlReq = jsonToXml(params)

  const resp = await fetch('https://api.mch.weixin.qq.com/pay/unifiedorder', {
    method: 'POST',
    headers: { 'Content-Type': 'text/xml' },
    body: xmlReq
  })

  const xmlResp = await resp.text()
  const jsonResp = xmlToJson(xmlResp)

  if (jsonResp.return_code !== 'SUCCESS' || jsonResp.result_code !== 'SUCCESS') {
    // 先在 Supabase 记录一个 pending 订单，让用户知道已尝试
    try {
      await upsertPayment(env.SUPABASE_URL, env.SUPABASE_SERVICE_KEY, user_id, outTradeNo, 'pending')
    } catch (e) {
      console.error('记录 pending 失败:', e)
    }

    return new Response(JSON.stringify({
      error: jsonResp.return_msg || jsonResp.err_code_des || '创建订单失败',
      detail: jsonResp
    }), {
      status: 400,
      headers: { 'Content-Type': 'application/json' }
    })
  }

  // 记录订单到 Supabase
  try {
    await upsertPayment(env.SUPABASE_URL, env.SUPABASE_SERVICE_KEY, user_id, outTradeNo, 'pending')
  } catch (e) {
    console.error('记录订单失败:', e)
  }

  return new Response(JSON.stringify({
    code_url: jsonResp.code_url,
    out_trade_no: outTradeNo
  }), {
    headers: { 'Content-Type': 'application/json' }
  })
}

async function handleWechatNotify(request, env) {
  const xmlText = await request.text()
  const params = xmlToJson(xmlText)

  // 验证签名
  const sign = params.sign
  delete params.sign
  const expectedSign = await md5SignAsync(params, env.WECHAT_API_KEY)

  if (sign !== expectedSign) {
    console.error('微信通知签名验证失败')
    return new Response(jsonToXml({
      return_code: 'FAIL',
      return_msg: '签名验证失败'
    }), {
      headers: { 'Content-Type': 'text/xml' }
    })
  }

  if (params.return_code !== 'SUCCESS' || params.result_code !== 'SUCCESS') {
    console.log('微信通知支付失败:', params.err_code, params.err_code_des)
    return new Response(jsonToXml({
      return_code: 'SUCCESS',
      return_msg: 'OK'
    }), {
      headers: { 'Content-Type': 'text/xml' }
    })
  }

  // 支付成功！更新 Supabase
  const outTradeNo = params.out_trade_no

  // 查找订单对应的 user_id
  try {
    const payment = await getPaymentByOrder(env.SUPABASE_URL, env.SUPABASE_SERVICE_KEY, outTradeNo)
    if (!payment) {
      console.error(`未找到订单: ${outTradeNo}`)
      return new Response(jsonToXml({
        return_code: 'FAIL',
        return_msg: '订单不存在'
      }), {
        headers: { 'Content-Type': 'text/xml' }
      })
    }

    await upsertPayment(env.SUPABASE_URL, env.SUPABASE_SERVICE_KEY, payment.user_id, outTradeNo, 'verified')
    console.log(`支付成功: user=${payment.user_id}, order=${outTradeNo}, fee=${params.total_fee}`)

    // 升级用户等级到 Plus
    try {
      await upsertUserTier(env.SUPABASE_URL, env.SUPABASE_SERVICE_KEY, payment.user_id, 'plus')
      console.log(`用户等级已升级: user=${payment.user_id} -> plus`)
    } catch (e) {
      console.error('升级用户等级失败:', e)
      return new Response(jsonToXml({
        return_code: 'FAIL',
        return_msg: '升级用户等级失败'
      }), {
        headers: { 'Content-Type': 'text/xml' }
      })
    }
  } catch (e) {
    console.error('更新支付状态失败:', e)
    return new Response(jsonToXml({
      return_code: 'FAIL',
      return_msg: '更新失败'
    }), {
      headers: { 'Content-Type': 'text/xml' }
    })
  }

  return new Response(jsonToXml({
    return_code: 'SUCCESS',
    return_msg: 'OK'
  }), {
    headers: { 'Content-Type': 'text/xml' }
  })
}

async function handleCheckOrder(request, env) {
  const url = new URL(request.url)
  const userId = url.searchParams.get('user_id')

  if (!userId) {
    return new Response(JSON.stringify({ error: '缺少 user_id' }), {
      status: 400, headers: { 'Content-Type': 'application/json' }
    })
  }

  try {
    // 查询用户等级
    const tierResp = await fetch(
      `${env.SUPABASE_URL}/rest/v1/user_tiers?user_id=eq.${encodeURIComponent(userId)}&select=tier`,
      {
        headers: {
          'apikey': env.SUPABASE_SERVICE_KEY,
          'Authorization': `Bearer ${env.SUPABASE_SERVICE_KEY}`
        }
      }
    )
    const tierData = await tierResp.json()
    const tier = tierData?.[0]?.tier || 'basic'

    // 查询最新支付状态（用于前端展示订单进度）
    const payResp = await fetch(
      `${env.SUPABASE_URL}/rest/v1/payments?user_id=eq.${encodeURIComponent(userId)}&select=status,out_trade_no,updated_at&order=updated_at.desc&limit=1`,
      {
        headers: {
          'apikey': env.SUPABASE_SERVICE_KEY,
          'Authorization': `Bearer ${env.SUPABASE_SERVICE_KEY}`
        }
      }
    )
    const payData = await payResp.json()

    return new Response(JSON.stringify({
      tier: tier,
      status: payData?.[0]?.status || 'none'
    }), {
      headers: { 'Content-Type': 'application/json' }
    })
  } catch (e) {
    return new Response(JSON.stringify({ error: e.message }), {
      status: 500, headers: { 'Content-Type': 'application/json' }
    })
  }
}

// ============================================================
//  入口
// ============================================================

export default {
  async fetch(request, env) {
    const url = new URL(request.url)
    const path = url.pathname

    // CORS headers
    const corsHeaders = {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type'
    }

    if (request.method === 'OPTIONS') {
      return new Response(null, { headers: corsHeaders })
    }

    try {
      let response

      if (path === '/create-order' && request.method === 'POST') {
        response = await handleCreateOrder(request, env)
      } else if (path === '/wechat-notify' && request.method === 'POST') {
        response = await handleWechatNotify(request, env)
      } else if (path === '/check-order' && request.method === 'GET') {
        response = await handleCheckOrder(request, env)
      } else {
        response = new Response(JSON.stringify({
          name: 'BilibiliAccountManager WeChat Pay Worker',
          endpoints: ['POST /create-order', 'POST /wechat-notify', 'GET /check-order']
        }), {
          headers: { 'Content-Type': 'application/json' }
        })
      }

      // 合并 CORS headers
      const mergedHeaders = new Headers(response.headers)
      for (const [k, v] of Object.entries(corsHeaders)) {
        mergedHeaders.set(k, v)
      }

      return new Response(response.body, {
        status: response.status,
        headers: mergedHeaders
      })
    } catch (e) {
      return new Response(JSON.stringify({ error: e.message }), {
        status: 500,
        headers: { 'Content-Type': 'application/json', ...corsHeaders }
      })
    }
  }
}
