package com.swissarmynoife.sdk;

/** Quickstart against a running http-admin (sak331-d). */
public final class Quickstart {
  public static void main(String[] args) throws Exception {
    String base = System.getenv().getOrDefault("SAK_HTTP", "http://127.0.0.1:8787");
    SakClient c = new SakClient(base);
    System.out.println("health=" + c.health());
    System.out.println("modules=" + c.listModules());
  }
}
