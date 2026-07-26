namespace SwissArmyNoife.Sdk.Tests;

public class ScaffoldTests
{
    [Fact]
    public void SdkInfo_HasName()
    {
        Assert.Equal("SwissArmyNoife.Sdk", SdkInfo.Name);
    }
}

