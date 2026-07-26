using SwissArmyNoife.Sdk;

var baseUrl = Environment.GetEnvironmentVariable("SAK_HTTP") ?? "http://127.0.0.1:8787";
var sak = new SakClient(baseUrl);
Console.WriteLine($"health={await sak.HealthAsync()}");
Console.WriteLine($"modules={await sak.ListModulesAsync()}");
